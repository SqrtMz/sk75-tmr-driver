//! Linux HID transport for SK75 TMR.
//!
//! Mirrors `common_hid_rs.exe` (hidapi-rs + tonic gRPC `hid.HidService`
//! on `http://127.0.0.1:3838`) and the JS `pK.sendMsg/commonMsg/readMsg`
//! layer: feature-report write + feature-report read with 10 ms pacing.
//! On Linux we talk to the kernel hidraw nodes directly via `hidapi`,
//! so no Windows `HidD_SetFeature` / Electron / iot_manager bridge is needed.

use anyhow::{anyhow, Context, Result};
use hidapi::{DeviceInfo, HidApi, HidDevice};
use std::time::Duration;

use crate::proto::{self, ReportKind};

pub const FEATURE_LEN: usize = 64;
// Original pacing (`pK` in index.5627ed60.js): `SEND_TIME=10`,
// `READ_TIME=10`; `sendMsg` sleeps *before* `___sendFeature` and `readMsg`
// sleeps *before* `___getFeature`. The live analog path overrides both with
// `_getMulitMagnetismCMD(254, 4, 1, 1)`, so it keeps a 1 ms round-trip.
pub const SEND_DELAY: Duration = Duration::from_millis(10);
pub const READ_DELAY: Duration = Duration::from_millis(10);
pub const LIVE_DELAY: Duration = Duration::from_millis(1);
pub const VENDOR_SLEEP: Duration = Duration::from_millis(100);
/// Extra wait before a post-write full refresh. Hardware probing shows the
/// keyboard remains busy for about 150-180 ms after the vendor sleep.
pub const POST_WRITE_SETTLE: Duration = Duration::from_millis(200);
/// The keyboard applies travel writes asynchronously. Hardware probing shows
/// it answers GET_FEATURE with the *request* payload for ~150-180 ms after a
/// write, then starts returning fresh data. These retries cover that window
/// for reads whose normal response begins with data (op 0/1/6/252/254 and
/// keymaps), so a too-early read cannot be mistaken for real values.
const ECHO_RETRIES: usize = 4;
const ECHO_RETRY_DELAY: Duration = Duration::from_millis(50);

/// One key's magnetism fields in raw units. `None` means "not known", which
/// the original treats as changed (`!("travel" in previousState)`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TravelState {
    pub press: Option<u16>,
    pub lift: Option<u16>,
    pub deadzone: Option<u16>,
}

/// Select the ops `setMagnetismInfoSimple` would actually send.
///
/// The original compares the edited key model with the model loaded from the
/// keyboard and pushes an op only when the value differs (or when the
/// previous model had no such field). Ops stay in the original order:
/// press `0`, release `1`, deadzone `6`.
pub fn changed_travel_ops(previous: TravelState, desired: TravelState) -> Vec<(u8, u16)> {
    let mut ops = Vec::with_capacity(3);
    for (op, prev, want) in [
        (proto::mag_op::TRAVEL, previous.press, desired.press),
        (proto::mag_op::LIFT_TRAVEL, previous.lift, desired.lift),
        (proto::mag_op::DEADZONE, previous.deadzone, desired.deadzone),
    ] {
        if let Some(value) = want {
            if prev != Some(value) {
                ops.push((op, value));
            }
        }
    }
    ops
}

pub struct Sk75 {
    dev: HidDevice,
    pub path: String,
}

pub fn enumerate_sk75(vid: u16, pid: u16) -> Result<Vec<DeviceInfo>> {
    let api = HidApi::new().context("hid_init failed")?;
    Ok(api
        .device_list()
        .filter(|d| d.vendor_id() == vid && d.product_id() == pid)
        .cloned()
        .collect())
}

/// Open the vendor-control interface.
///
/// The keyboard exposes several hidraw nodes (boot keyboard usage 6/page 1
/// plus vendor control usage 2/page 0xFFFF). We prefer usage page 0xFFFF
/// when the backend reports it, otherwise the interface with a 64-byte
/// feature report, otherwise the first matching VID:PID node.
pub fn open_sk75(vid: u16, pid: u16, path: Option<&str>, iface: Option<i32>) -> Result<Sk75> {
    let api = HidApi::new().context("hid_init failed")?;
    let mut cands: Vec<&DeviceInfo> = api
        .device_list()
        .filter(|d| d.vendor_id() == vid && d.product_id() == pid)
        .collect();
    if cands.is_empty() {
        return Err(anyhow!("no HID device {:04x}:{:04x} found", vid, pid));
    }
    if let Some(p) = path {
        cands.retain(|d| d.path().to_string_lossy() == p);
    }
    if let Some(n) = iface {
        cands.retain(|d| d.interface_number() == n);
    }
    if cands.is_empty() {
        return Err(anyhow!("no HID device matches path/interface filter"));
    }
    // Prefer the vendor-control interface (JS Qf: usage 2 / page 0xFFFF / iface 2).
    // hidraw3 (iface 2, usage 2, page 0xFFFF) is the 20-byte control collection;
    // hidraw2/iface 1 also exposes a vendor collection but is not the control path.
    let pick: DeviceInfo = cands
        .iter()
        .find(|d| d.interface_number() == 2 && d.usage_page() == crate::proto::VENDOR_USAGE_PAGE)
        .or_else(|| cands.iter().find(|d| d.interface_number() == 2))
        .or_else(|| {
            cands
                .iter()
                .find(|d| d.usage_page() == crate::proto::VENDOR_USAGE_PAGE)
        })
        .or_else(|| cands.iter().find(|d| d.usage_page() == 0xFF00))
        .or(cands.first())
        .cloned()
        .cloned()
        .ok_or_else(|| anyhow!("candidate vanished"))?;
    let dev_path = pick.path().to_string_lossy().into_owned();
    let dev = api.open_path(&std::ffi::CString::new(dev_path.clone())?)?;
    log::info!(
        "opened {:04x}:{:04x} path={} usage={:#x} page={:#x} iface={} '{}'",
        vid,
        pid,
        dev_path,
        pick.usage(),
        pick.usage_page(),
        pick.interface_number(),
        pick.product_string().unwrap_or("")
    );
    Ok(Sk75 {
        dev,
        path: dev_path,
    })
}

impl Sk75 {
    /// JS `sendMsg(t, Bit7|Bit8)`: checksum-encode then feature-write.
    /// `payload` is the raw command (e.g. 8 B or 64 B, first byte = FEA_CMD).
    /// Report ID 0 is prepended for hidapi.
    pub fn send_msg(&self, payload: &[u8], kind: ReportKind) -> Result<()> {
        self.send_msg_with_timing(payload, kind, SEND_DELAY)
    }

    /// Timing-explicit variant matching JS `sendMsg(t, kind, delay)`: the
    /// delay is taken **before** the feature report is written. Normal
    /// traffic uses 10 ms; live op-254/255 polling uses 1 ms.
    pub fn send_msg_with_timing(
        &self,
        payload: &[u8],
        kind: ReportKind,
        send_delay: Duration,
    ) -> Result<()> {
        let enc = proto::encode_cmd(payload.to_vec(), kind);
        let mut report = Vec::with_capacity(enc.len() + 1);
        report.push(0u8); // report ID
        report.extend_from_slice(&enc);
        // Pad short commands to a full 64 B feature (firmware tolerates both,
        // full-length is what the JS 64 B paths use).
        while report.len() < FEATURE_LEN + 1 {
            report.push(0);
        }
        if report.len() != FEATURE_LEN + 1 {
            return Err(anyhow!("feature payload must be 65 bytes"));
        }
        if !send_delay.is_zero() {
            std::thread::sleep(send_delay);
        }
        self.dev.send_feature_report(&report)?;
        Ok(())
    }

    /// JS `commonMsg(t, kind)`: `sendMsg` then `readMsg` (feature read).
    /// Returns the 64 B report body with the leading report ID stripped.
    pub fn common_msg(&self, payload: &[u8], kind: ReportKind) -> Result<Vec<u8>> {
        self.common_msg_with_timing(payload, kind, SEND_DELAY, READ_DELAY)
    }

    pub fn common_msg_with_timing(
        &self,
        payload: &[u8],
        kind: ReportKind,
        send_delay: Duration,
        read_delay: Duration,
    ) -> Result<Vec<u8>> {
        self.send_msg_with_timing(payload, kind, send_delay)?;
        let mut buf = vec![0u8; FEATURE_LEN + 1];
        buf[0] = 0;
        // JS `readMsg` waits before `___getFeature`; without this the
        // firmware can still return the previous command's report.
        if !read_delay.is_zero() {
            std::thread::sleep(read_delay);
        }
        let n = self.dev.get_feature_report(&mut buf)?;
        buf.truncate(n.max(1));
        Ok(if buf.is_empty() {
            buf
        } else {
            buf[1..].to_vec()
        })
    }

    /// True when `response` is just the last SET_FEATURE payload echoed back.
    /// Only valid for commands whose normal response starts with data rather
    /// than with the command byte (multi-magnetism and keymap reads).
    fn response_is_request_echo(response: &[u8], request: &[u8]) -> bool {
        let n = request.len().min(response.len());
        n >= 8 && response[..n] == request[..n]
    }

    /// Read a data-bearing command, retrying if the keyboard is still busy
    /// applying a previous write. Returns an error instead of passing the
    /// echoed request off as data.
    pub(crate) fn common_msg_data(&self, payload: &[u8], kind: ReportKind) -> Result<Vec<u8>> {
        self.common_msg_data_with_timing(payload, kind, SEND_DELAY, READ_DELAY)
    }

    fn common_msg_data_with_timing(
        &self,
        payload: &[u8],
        kind: ReportKind,
        send_delay: Duration,
        read_delay: Duration,
    ) -> Result<Vec<u8>> {
        let request = proto::encode_cmd(payload.to_vec(), kind);
        for attempt in 0..=ECHO_RETRIES {
            let r = self.common_msg_with_timing(payload, kind, send_delay, read_delay)?;
            if !Self::response_is_request_echo(&r, &request) {
                return Ok(r);
            }
            if attempt < ECHO_RETRIES {
                std::thread::sleep(ECHO_RETRY_DELAY);
            }
        }
        Err(anyhow!(
            "keyboard echoed the read request instead of returning data (still applying a write); retry the read"
        ))
    }

    pub fn vendor_sleep(dur: Option<Duration>) {
        std::thread::sleep(dur.unwrap_or(VENDOR_SLEEP));
    }

    // ---- High-level SK75 TMR operations (7a5b12c9.js) ----

    /// `whoAmI`: `commonMsg([143], Bit7)` -> u32 LE at bytes 1..4.
    pub fn device_id(&self) -> Result<Option<u32>> {
        let r = self.common_msg(&[proto::cmd::GET_USB_VERSION], ReportKind::Bit7)?;
        Ok(proto::parse_device_id(&r))
    }

    /// `getUSBVersion`: `commonMsg([143], Bit7)` -> `r[8]<<8|r[7]`.
    pub fn usb_version(&self) -> Result<Option<u16>> {
        let r = self.common_msg(&[proto::cmd::GET_USB_VERSION], ReportKind::Bit7)?;
        Ok(proto::parse_usb_version(&r))
    }

    /// `getRFVersion`: `commonMsg([128], Bit7)` -> `r[2]<<8|r[1]`, with 0
    /// meaning "not populated" (wired-only firmware). The original driver
    /// stores this as `deviceType.version.rf` and the calibration overlay
    /// prefers it over `version.usb`.
    pub fn rf_version(&self) -> Result<Option<u16>> {
        let r = self.common_msg(&[proto::cmd::GET_RF_VERSION], ReportKind::Bit7)?;
        if r.len() < 3 || r[0] != proto::cmd::GET_RF_VERSION {
            return Ok(None);
        }
        let n = ((r[2] as u16) << 8) | r[1] as u16;
        Ok((n != 0).then_some(n))
    }

    /// `getPrecisionEnum` via `FEA 230`: `[230,0xAA,precision]` or None.
    pub fn precision(&self) -> Result<Option<proto::Precision>> {
        let mut req = vec![0u8; 64];
        req[0] = proto::cmd::GET_FEATURE_LIST;
        let r = self.common_msg(&req, ReportKind::Bit7)?;
        Ok(proto::parse_feature_list(&r))
    }

    /// Effective travel scale: precision-derived when available,
    /// else version fallback (`scale_from_version`).
    pub fn travel_scale(&self) -> Result<u32> {
        if let Some(p) = self.precision()? {
            return Ok(p.scale());
        }
        // The original prefers the RF version over USB for the fallback
        // (`version.rf ?? version.usb`). `rf_version()` already maps 0 to
        // None, matching `i || r` in `get磁轴行程步进倍数`. Version reads
        // are best-effort so a missing RF reply does not hide USB, but if
        // both fail return an error so callers can fall back to the scale
        // already shown in the device info.
        let usb_res = self.usb_version();
        let rf_res = self.rf_version();
        let usb = usb_res.as_ref().ok().and_then(|v| *v).filter(|v| *v != 0);
        let rf = rf_res.as_ref().ok().and_then(|v| *v).filter(|v| *v != 0);
        if usb_res.is_err() && rf_res.is_err() {
            return Err(anyhow!("could not read USB/RF version for travel scale"));
        }
        Ok(proto::scale_from_version(usb, rf))
    }

    /// One `_getMulitMagnetismCMD(op, pages)` round: for each 0..pages,
    /// `commonMsg([229, op, 1, page], Bit7)` and concatenate the 64 B bodies.
    /// Mirrors JS failure semantics (zeros on timeout) via anyhow error instead.
    /// Normal configuration reads use the driver's 10 ms send/read pacing.
    pub fn get_multi_magnetism(&self, op: u8, pages: u8) -> Result<Vec<u8>> {
        self.get_multi_magnetism_with_timing(op, pages, SEND_DELAY, READ_DELAY)
    }

    /// Timing-explicit variant. `_getJiaoZhunXinXi` (live op 254/255) calls
    /// `_getMulitMagnetismCMD(op, 4, 1, 1)`, so live polling passes
    /// `LIVE_DELAY` on both sides.
    pub fn get_multi_magnetism_with_timing(
        &self,
        op: u8,
        pages: u8,
        send_delay: Duration,
        read_delay: Duration,
    ) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(pages as usize * FEATURE_LEN);
        for page in 0..pages {
            let req = proto::build_get_multi_magnetism(op, page);
            let r =
                self.common_msg_data_with_timing(&req, ReportKind::Bit7, send_delay, read_delay)?;
            out.extend_from_slice(&r);
        }
        Ok(out)
    }

    /// `setCurrentProfile`: `sendMsg([4, t], Bit7)` + vendor sleep
    /// (`7a5b12c9.js setCurrentProfile`, FEA_CMD_SET_PROFILE=4). The driver's
    /// profile-change watcher (`___check当前按键层索引变化`) always pairs this
    /// with re-fetching that profile's data.
    pub fn set_profile(&self, profile: u8) -> Result<()> {
        self.send_msg(&[proto::cmd::SET_PROFILE, profile], ReportKind::Bit7)?;
        Self::vendor_sleep(None);
        Ok(())
    }

    /// `getCurrentProfile`: `commonMsg([132], Bit7)` -> `r[1]`
    /// (FEA_CMD_GET_PROFILE=132). Driver startup syncs the UI layer
    /// selector to this ("const t = await this.dev.getCurrentProfile()??0").
    pub fn get_profile(&self) -> Result<Option<u8>> {
        let r = self.common_msg(&[proto::cmd::GET_PROFILE], ReportKind::Bit7)?;
        if r.len() < 2 || r[0] != proto::cmd::GET_PROFILE {
            return Ok(None);
        }
        Ok(Some(r[1]))
    }

    /// `getReportRate`: `commonMsg([131], Bit7)` -> `e[2]` 0..6 maps to Hz.
    pub fn report_rate_hz(&self) -> Result<Option<u32>> {
        let r = self.common_msg(&[proto::cmd::GET_REPORT], ReportKind::Bit7)?;
        if r.len() < 3 {
            return Ok(None);
        }
        Ok(match r[2] {
            0 => Some(8000),
            1 => Some(4000),
            2 => Some(2000),
            3 => Some(1000),
            4 => Some(500),
            5 => Some(250),
            6 => Some(125),
            _ => None,
        })
    }

    /// Live per-key analog travel for every matrix slot (op 254, 4 pages = 256 B
    /// -> 128 u16 LE). Returns raw units; divide by `travel_scale()` for mm.
    /// JS `_getJiaoZhunXinXi` (+ `_decodeMultiMagnetismTwoByteList`).
    pub fn live_travel_raw(&self) -> Result<Vec<u16>> {
        let blob = self.get_multi_magnetism_with_timing(
            proto::mag_op::LIVE_TRAVEL,
            4,
            LIVE_DELAY,
            LIVE_DELAY,
        )?;
        Ok(proto::decode_u16le_list(&blob))
    }

    /// Configured activation travel per slot: press (op 0) + release
    /// (op 1), each 4 pages = 256 B -> 128 u16 LE (one per matrix slot).
    /// JS `getMultiMagnetismInfo`: `触发行程` = `_getMulitMagnetismCMD(0, 4)`,
    /// `抬起行程` = `_getMulitMagnetismCMD(1, 4)`, both through
    /// `_decodeMultiMagnetismTwoByteList` and divided by the travel-step
    /// scale for mm. Raw units here; divide by `travel_scale()`.
    pub fn press_lift_travel_raw(&self) -> Result<(Vec<u16>, Vec<u16>)> {
        let press = self.get_multi_magnetism(proto::mag_op::TRAVEL, 4)?;
        let lift = self.get_multi_magnetism(proto::mag_op::LIFT_TRAVEL, 4)?;
        Ok((
            proto::decode_u16le_list(&press),
            proto::decode_u16le_list(&lift),
        ))
    }

    /// Configured per-key deadzone (op 6, 4 pages). The original release
    /// setter clamps to full travel minus this value; missing/unreadable
    /// values fall back to the driver's 0.3 mm default in the UI.
    pub fn deadzone_travel_raw(&self) -> Result<Vec<u16>> {
        let blob = self.get_multi_magnetism(proto::mag_op::DEADZONE, 4)?;
        Ok(proto::decode_u16le_list(&blob))
    }

    /// Write the selected key's magnetism values in one apply batch.
    ///
    /// Mirrors `setMagnetismInfoSimple(newState, previousState)`: only fields
    /// whose raw value differs from the previously loaded key are sent.
    /// Ops keep the original order: press travel (0), release travel (1),
    /// deadzone (6). Only the final changed op carries `last=1`; a single
    /// 100 ms vendor sleep follows a non-empty batch. If nothing changed,
    /// no packet and no vendor sleep is emitted, exactly like the original.
    ///
    /// Returns the op codes actually sent. The switch type is a separate
    /// op-252 write after this batch.
    pub fn write_magnetism(
        &self,
        slot: usize,
        previous: TravelState,
        desired: TravelState,
    ) -> Result<Vec<u8>> {
        if slot > 127 {
            return Err(anyhow!("slot {slot} out of range (0..127)"));
        }
        let ops = changed_travel_ops(previous, desired);
        for req in proto::build_magnetism_batch(slot as u8, &ops) {
            self.send_msg(&req, ReportKind::Bit7)?;
        }
        if !ops.is_empty() {
            Self::vendor_sleep(None);
        }
        Ok(ops.into_iter().map(|(op, _)| op).collect())
    }

    /// Travel calibration switches (cmds 28 / 30).
    /// JS `_setZuiXiaoXingChengJiaoZhun` / `_setZuiDaXingChengJiaoZhun`.
    pub fn set_min_travel_cal(&self, enable: bool) -> Result<()> {
        self.send_msg(
            &[proto::cmd::SET_MAGNETISM_CAL, u8::from(enable)],
            ReportKind::Bit7,
        )?;
        Self::vendor_sleep(None);
        Ok(())
    }
    pub fn set_max_travel_cal(&self, enable: bool) -> Result<()> {
        self.send_msg(
            &[
                proto::cmd::SET_MAGNETISM_MAXIMUM_CALIBRATION,
                u8::from(enable),
            ],
            ReportKind::Bit7,
        )?;
        Self::vendor_sleep(None);
        Ok(())
    }

    /// `setJiaoZhunKaiGuan(false)`: max-cal off.
    pub fn calibrate_stop(&self) -> Result<()> {
        self.set_max_travel_cal(false)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(press: Option<u16>, lift: Option<u16>, deadzone: Option<u16>) -> TravelState {
        TravelState {
            press,
            lift,
            deadzone,
        }
    }

    #[test]
    fn only_changed_travel_fields_are_sent_in_original_order() {
        let prev = state(Some(200), Some(280), Some(30));
        // Press-only edit.
        assert_eq!(
            changed_travel_ops(prev, state(Some(250), Some(280), Some(30))),
            vec![(proto::mag_op::TRAVEL, 250)]
        );
        // Release-only edit (this is what toggle-off produces when press was
        // unchanged but the stored release differed).
        assert_eq!(
            changed_travel_ops(prev, state(Some(200), Some(200), Some(30))),
            vec![(proto::mag_op::LIFT_TRAVEL, 200)]
        );
        // Deadzone-only edit.
        assert_eq!(
            changed_travel_ops(prev, state(Some(200), Some(280), Some(50))),
            vec![(proto::mag_op::DEADZONE, 50)]
        );
        // Everything changes: original 0, 1, 6 order.
        assert_eq!(
            changed_travel_ops(prev, state(Some(210), Some(290), Some(40))),
            vec![
                (proto::mag_op::TRAVEL, 210),
                (proto::mag_op::LIFT_TRAVEL, 290),
                (proto::mag_op::DEADZONE, 40)
            ]
        );
    }

    #[test]
    fn busy_request_echo_is_detected_but_travel_data_is_not() {
        let req = proto::build_get_multi_magnetism(proto::mag_op::TRAVEL, 2);
        let mut echo = req.clone();
        echo.resize(FEATURE_LEN, 0);
        assert!(Sk75::response_is_request_echo(&echo, &req));

        // Real op-0 page data begins with press-travel u16s, not the request.
        let mut data = vec![50u8, 0, 50, 0, 50, 0, 50, 0];
        data.resize(FEATURE_LEN, 0);
        assert!(!Sk75::response_is_request_echo(&data, &req));

        // Short/garbage responses cannot be classified as an echo.
        assert!(!Sk75::response_is_request_echo(&[229, 0, 1], &req));
    }

    #[test]
    fn unchanged_or_unread_values_follow_js_semantics() {
        let prev = state(Some(200), Some(280), Some(30));
        // No changes -> empty batch (no packet, no vendor sleep).
        assert!(changed_travel_ops(prev, prev).is_empty());
        // Missing previous fields count as changed, like `!("travel" in e)`.
        assert_eq!(
            changed_travel_ops(
                TravelState::default(),
                state(Some(200), Some(280), Some(30))
            ),
            vec![
                (proto::mag_op::TRAVEL, 200),
                (proto::mag_op::LIFT_TRAVEL, 280),
                (proto::mag_op::DEADZONE, 30)
            ]
        );
    }
}
