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
// Original pacing: most traffic uses 10 ms, but the live analog path
// passes 1 ms explicitly (`_getMulitMagnetismCMD(254,4,1,1)`) and the live
// loop re-polls with no sleep, so live reads always use 1 ms here.
pub const SEND_DELAY: Duration = Duration::from_millis(1);
pub const READ_DELAY: Duration = Duration::from_millis(1);
pub const VENDOR_SLEEP: Duration = Duration::from_millis(100);

pub struct Sk75 {
    dev: HidDevice,
    pub vid: u16,
    pub pid: u16,
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
        vid,
        pid,
        path: dev_path,
    })
}

impl Sk75 {
    /// Low-level feature write. `hidapi::HidDevice::send_feature_report`
    /// prepends the report ID itself, so `payload` must already start with
    /// the report ID byte (0 for SK75). Payloads longer than 64 B are
    /// rejected by the firmware; longer JS buffers are chunked by callers.
    pub fn send_feature(&self, payload: &[u8]) -> Result<()> {
        if payload.is_empty() || payload.len() > FEATURE_LEN + 1 {
            return Err(anyhow!("feature payload must be 1..=65 bytes"));
        }
        self.dev.send_feature_report(payload)?;
        std::thread::sleep(SEND_DELAY);
        Ok(())
    }

    /// Low-level feature read into a 65-byte buffer (report ID + 64 B).
    pub fn get_feature(&self, report_id: u8) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; FEATURE_LEN + 1];
        buf[0] = report_id;
        std::thread::sleep(READ_DELAY);
        let n = self.dev.get_feature_report(&mut buf)?;
        buf.truncate(n.max(1));
        Ok(buf)
    }

    /// JS `sendMsg(t, Bit7|Bit8)`: checksum-encode then feature-write.
    /// `payload` is the raw command (e.g. 8 B or 64 B, first byte = FEA_CMD).
    /// Report ID 0 is prepended for hidapi.
    pub fn send_msg(&self, payload: &[u8], kind: ReportKind) -> Result<()> {
        self.send_msg_with_timing(payload, kind, SEND_DELAY, READ_DELAY)
    }

    /// Timing-explicit variant. The original driver paces most traffic at
    /// `SEND_TIME=10ms`/`READ_TIME=10ms` (`pK`), but the live analog path
    /// `_getMulitMagnetismCMD(254,4,1,1)` passes `1,1` explicitly and the
    /// `getJiaoZhunXinXiLoop` re-polls with no inter-poll sleep — i.e. the
    /// live poll runs back-to-back at HID round-trip speed, not 10 Hz.
    pub fn send_msg_with_timing(
        &self,
        payload: &[u8],
        kind: ReportKind,
        send_delay: Duration,
        _read_delay: Duration,
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
        self.dev.send_feature_report(&report)?;
        std::thread::sleep(send_delay);
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
        self.send_msg_with_timing(payload, kind, send_delay, read_delay)?;
        let mut buf = vec![0u8; FEATURE_LEN + 1];
        buf[0] = 0;
        std::thread::sleep(read_delay);
        let n = self.dev.get_feature_report(&mut buf)?;
        buf.truncate(n.max(1));
        Ok(if buf.is_empty() { buf } else { buf[1..].to_vec() })
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
        let v = self.usb_version()?.unwrap_or(0);
        Ok(proto::scale_from_version(
            if v == 0 { None } else { Some(v) },
            None,
        ))
    }

    /// One `_getMulitMagnetismCMD(op, pages)` round: for each 0..pages,
    /// `commonMsg([229, op, 1, page], Bit7)` and concatenate the 64 B bodies.
    /// Mirrors JS failure semantics (zeros on timeout) via anyhow error instead.
    pub fn get_multi_magnetism(&self, op: u8, pages: u8) -> Result<Vec<u8>> {
        self.get_multi_magnetism_with_timing(op, pages, SEND_DELAY, READ_DELAY)
    }

    /// Fast variant matching `_getMulitMagnetismCMD(op, pages, 1, 1)` used by
    /// `_getJiaoZhunXinXi` (live op 254/255): 1 ms pacing per round-trip.
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
            // Trim the 8 B builder to its significant prefix; send_msg pads.
            let r = self.common_msg_with_timing(&req, ReportKind::Bit7, send_delay, read_delay)?;
            out.extend_from_slice(&r);
        }
        Ok(out)
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

    /// Live analog travel for every matrix slot (op 254, 4 pages = 256 B
    /// -> 128 u16 LE). Returns raw units; divide by `travel_scale()` for mm.
    /// JS `_getJiaoZhunXinXi` (+ `_decodeMultiMagnetismTwoByteList`).
    pub fn live_travel_raw(&self) -> Result<Vec<u16>> {
        let blob = self.get_multi_magnetism_with_timing(
            proto::mag_op::LIVE_TRAVEL,
            4,
            SEND_DELAY,
            READ_DELAY,
        )?;
        Ok(proto::decode_u16le_list(&blob))
    }

    /// Live dynamic analog values (op 255, same layout).
    pub fn live_dynamic_raw(&self) -> Result<Vec<u16>> {
        let blob = self.get_multi_magnetism_with_timing(
            proto::mag_op::LIVE_DYNAMIC,
            4,
            SEND_DELAY,
            READ_DELAY,
        )?;
        Ok(proto::decode_u16le_list(&blob))
    }

    /// Enable/disable unsolicited magnetism streaming (`SET 27`).
    /// JS `setKeyMagnetismReport(t)`: `[27, Number(t)]` + Bit7.
    pub fn set_magnetism_report(&self, enable: bool) -> Result<()> {
        self.send_msg(
            &[proto::cmd::SET_MAGNETISM_REPOR, u8::from(enable)],
            ReportKind::Bit7,
        )?;
        Self::vendor_sleep(None);
        Ok(())
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
            &[proto::cmd::SET_MAGNETISM_MAXIMUM_CALIBRATION, u8::from(enable)],
            ReportKind::Bit7,
        )?;
        Self::vendor_sleep(None);
        Ok(())
    }

    /// Full calibration sequence from `setJiaoZhunKaiGuan(true)`:
    /// min-cal on, wait 2 s, min-cal off, max-cal on.
    pub fn calibrate_start(&self) -> Result<()> {
        self.set_min_travel_cal(true)?;
        std::thread::sleep(Duration::from_millis(2000));
        self.set_min_travel_cal(false)?;
        self.set_max_travel_cal(true)?;
        Ok(())
    }
    /// `setJiaoZhunKaiGuan(false)`: max-cal off.
    pub fn calibrate_stop(&self) -> Result<()> {
        self.set_max_travel_cal(false)?;
        Ok(())
    }
}
