//! SK75 TMR vendor HID protocol.
//!
//! Reverse-engineered from `womier-driver.exe` (WOMIER Driver 3.2.15,
//! NSIS -> `app-32.7z` -> Electron `resources/app`, JS `dist/js/*.js`
//! + Rust helpers `iot_manager/common_hid_rs.exe`, `iot_manager_rs.exe`).
//!
//! REA evidence (session retained):
//! - `open_binary` womier-driver.exe sha256
//!   2a01ddb1f7a1d4c7d75703b07d188c85bece74913bab39e67494378e55116826 (NSIS stub, 105996280 B)
//! - `binary_overview` (Hopper): 7 segments, 105 procedures, 38 strings
//! - `inspect_artifact` summary, `inspect_rust_binary` for
//!   common_hid_rs.exe (619fa555b598c058babb8dc11a242f5be6aec761b586dc3694362bc17ccec97a, 3073600 B)
//!   and iot_manager_rs.exe etc.
//! - `analyze_javascript_application` on `resources/app`
//!   (ev_ccb640c41ec213061a7afdd7f27a3b5386df1c06ba5ea09af268c7c00885a016,
//!   1138 JS files, 8389 findings) + summary view
//!   ev_f5d90268a497fcdbae4ba35e7d09350dc9de30cb99f801cbc9b83a745e9bbe81
//! - `inspect_managed_artifact` (common_hid_rs.exe = not-managed),
//!   `inspect_pe_resources` (PE32+, no resources).
//!
//! Key JS sources:
//! - `dist/js/7a5b12c9.js` class `at` (CommonKeyboard): FEA_CMD_* table,
//!   `setKeyMagnetismReport` (27), `_setZuiXiaoXingChengJiaoZhun` (28),
//!   `_setZuiDaXingChengJiaoZhun` (30), `_getJiaoZhunXinXi` (op 254),
//!   `_getMulitMagnetismCMD` (229), `_decodeMultiMagnetismTwoByteList`.
//! - `dist/js/60ee4367.js` class `H`: `FEA_CMD_GET_FEATURE_LIST=230`,
//!   precision handling, `setMultiMagnetismInfo` / travel scaling.
//! - `dist/js/index.5627ed60.js`: transport (`ki.Bit7=0, Bit8=1, None=2`,
//!   `Wi=sum`, `___encodeCmd` checksum, `SEND_TIME=10, READ_TIME=10,
//!   defaultVendorSleepTime=100`), device DB `VT`/`DT`
//!   (SK75 TMR: vid 12625 pid 20528, ids 2518 + 3804), `Qf` WebHID filters
//!   (vendor control: usage 2, usagePage 65535/0xFFFF).
//! - `dist/js/7de29f04.js` (`Ry5088_womiersk75he_8k_8k`, id 2518) and
//!   `74d1dc6b.js` (`Ry5088_womier_sk75he_europe_3m_8k_8k`, id 3804):
//!   `defaultMatrix` (512 u8 = 128 slots x 4 B).
//! - `common_hid_rs.exe` strings: `hid.HidService/SendFeatureReport`,
//!   `/GetFeatureReport`, `hidapi-rs`, `HidD_SetFeature/GetFeature`,
//!   gRPC `http://127.0.0.1:3838`, `DeviceFilter{vendor_id,product_id,
//!   usage,usage_page,interface_number,reports}`.

/// Vendor / product IDs for SK75 TMR (decimal + hex).
/// JS: `{id:2518,vid:12625,pid:20528,..."ry5088_womiersk75he_8k_8k",displayName:"SK75 TMR"}`
/// and `{id:3804,vid:12625,pid:20528,..."ry5088_womier_sk75he_europe_3m_8k_8k"}`.
pub const VID_RONGYUAN: u16 = 0x3151; // 12625
pub const PID_SK75_TMR: u16 = 0x5030; // 20528

/// Per-key activation-travel limits/defaults from the original driver
/// (`index.5627ed60.js` `createState_行程设置` + `设置按下`/`设置抬起`).
/// Press/lift minimum is 0.1 mm; defaults are 2.0 / 2.8 mm; the fallback
/// deadzone is 0.3 mm. Full travel is 3.3 mm for firmware >= 768 and
/// 4.0 mm below that (`选中按键行程最大值`).
pub const TRAVEL_MIN_MM: f32 = 0.1;
pub const TRAVEL_DEFAULT_PRESS_MM: f32 = 2.0;
pub const TRAVEL_DEFAULT_LIFT_MM: f32 = 2.8;
pub const TRAVEL_DEFAULT_DEADZONE_MM: f32 = 0.3;
/// Original `get死区()` clamps the selected key's deadzone to 0..=1 mm
/// before subtracting it from the slider maximum.
pub const TRAVEL_MAX_DEADZONE_MM: f32 = 1.0;
/// The deadzone editor's own maximum is 4 mm below firmware 768
/// (`createState_行程设置.死区` setter / `get死区最大值`), even though the
/// travel cap still uses the 0..=1 mm `get死区()` clamp above.
pub const TRAVEL_MAX_DEADZONE_LEGACY_MM: f32 = 4.0;
pub const TRAVEL_MAX_MM: f32 = 3.3;
pub const TRAVEL_MAX_MM_LEGACY: f32 = 4.0;

fn is_legacy_firmware(usb: Option<u16>, rf: Option<u16>) -> bool {
    matches!(rf.or(usb), Some(v) if v != 0 && v < 768)
}

/// Full travel for the current firmware. The original prefers the RF
/// version over USB and uses 4.0 mm only below firmware 768.
pub fn travel_max_mm(usb: Option<u16>, rf: Option<u16>) -> f32 {
    if is_legacy_firmware(usb, rf) {
        TRAVEL_MAX_MM_LEGACY
    } else {
        TRAVEL_MAX_MM
    }
}

/// Per-key deadzone slider maximum from the original
/// `createState_行程设置` setter: 4.0 mm below firmware 768, otherwise
/// 1.0 mm.
pub fn deadzone_max_mm(usb: Option<u16>, rf: Option<u16>) -> f32 {
    if is_legacy_firmware(usb, rf) {
        TRAVEL_MAX_DEADZONE_LEGACY_MM
    } else {
        TRAVEL_MAX_DEADZONE_MM
    }
}

/// Convert millimetres to raw travel units exactly like the original JS:
///
/// ```js
/// const l = value * get磁轴行程步进倍数();
/// c = [l & 255, l >> 8];
/// ```
///
/// `value` is a JS double and the bitwise ops truncate toward zero (the
/// original does not round). The f32 input is first normalised to six
/// decimal places so decimal constants such as 2.8/3.3 do not lose a unit
/// to their binary f32 representation; e.g. `0.29 * 100` still truncates
/// to 28, exactly as the original driver writes it.
pub fn mm_to_raw(mm: f32, scale: u32) -> u16 {
    let mm = ((mm as f64) * 1_000_000.0).round() / 1_000_000.0;
    (mm * scale.max(1) as f64)
        .trunc()
        .clamp(0.0, u16::MAX as f64) as u16
}

/// Vendor-control HID selector used by the old WebHID path (`Qf`):
/// `{usage:2, usagePage:65535, interfaceNumber:2}` for pid 16391+ etc.
/// Modern iot_manager path enumerates the same vendor page via
/// `IotDeviceFilter{usage,usage_page}` + feature reports.
pub const VENDOR_USAGE_PAGE: u16 = 0xFFFF;

/// Report-type selector (`ki` in index.js): NOT a HID report ID,
/// selects where the checksum byte goes.
#[allow(dead_code)] // Bit8/None are retained and exercised by protocol tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ReportKind {
    Bit7 = 0,
    Bit8 = 1,
    None = 2,
}

/// Keyboard feature commands (`FEA_CMD_*` in 7a5b12c9.js `class at`).
#[allow(dead_code)]
pub mod cmd {
    pub const SET_RESET: u8 = 1;
    pub const SET_REPORT: u8 = 3;
    pub const SET_PROFILE: u8 = 4;
    pub const SET_DEBOUNCE: u8 = 6;
    pub const SET_LEDPARAM: u8 = 7;
    pub const SET_SLEDPARAM: u8 = 8;
    pub const SET_KBOPTION: u8 = 9;
    pub const SET_KEYMATRIX: u8 = 10;
    pub const SET_MACRO: u8 = 11;
    pub const SET_USERPIC: u8 = 12;
    pub const SET_FN: u8 = 16;
    pub const SET_SLEEPTIME: u8 = 17;
    pub const SET_USERGIF: u8 = 18;
    pub const SET_CMD_AUTOOSEN: u8 = 23;
    pub const SET_MAGNETISM_REPOR: u8 = 27;
    pub const SET_MAGNETISM_CAL: u8 = 28;
    pub const SET_MAGNETISM_MAXIMUM_CALIBRATION: u8 = 30;
    pub const SET_MULTI_MAGNETISM: u8 = 101;
    pub const GET_RF_VERSION: u8 = 128;
    pub const GET_REPORT: u8 = 131;
    pub const GET_PROFILE: u8 = 132;
    pub const GET_USB_VERSION: u8 = 143;
    pub const GET_MULTI_MAGNETISM: u8 = 229;
    pub const GET_FEATURE_LIST: u8 = 230;
}

/// Multi-magnetism sub-operations (`et` enum in 7a5b12c9.js).
#[allow(dead_code)]
pub mod mag_op {
    pub const TRAVEL: u8 = 0; // press travel
    pub const LIFT_TRAVEL: u8 = 1; // release travel
    pub const RT_PRESS: u8 = 2; // rapid-trigger press travel
    pub const RT_LIFT: u8 = 3; // rapid-trigger release travel
    pub const DYNAMIC_START: u8 = 4; // dynamic keystroke start travel
    pub const MT_HOLD_MS_DIV10: u8 = 5; // MT hold time (/10)
    pub const DEADZONE: u8 = 6; // dead zone
    pub const KEY_MODE: u8 = 7; // key mode
    pub const DKS_FLAGS: u8 = 8; // dynamic keystroke option flags
    pub const SNAP_MAP: u8 = 9; // Snap mapping
    pub const ALL_DKS_FLAGS: u8 = 10; // all-key dynamic option flags
    /// Live per-key analog travel (u16 LE per matrix slot). THE analog readout.
    /// JS: `_getJiaoZhunXinXi` = `_getMulitMagnetismCMD(254,4,1,1)` + LE u16 decode.
    pub const LIVE_TRAVEL: u8 = 254; // live key-press travel value
    /// Live per-key dynamic analog value (u16 LE per matrix slot).
    #[allow(dead_code)] // op 255 is documented but not shown in the current UI.
    pub const LIVE_DYNAMIC: u8 = 255; // live key-press dynamic value
    pub const AXIS_TYPE: u8 = 252; // switch (axis) type
    pub const TOP_DEADZONE: u8 = 251; // top dead zone
}

/// Precision enum (`am` in index.js): `Precision001=0, Precision0005=1, Precision0001=2`.
/// JS `getPrecisionEnum` reads `FEA 230` response `[230, 0xAA, precision]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    P001,  // 0.01mm
    P0005, // 0.005mm
    P0001, // 0.001mm
}

impl Precision {
    /// Travel scale factor (raw units per mm), JS travel-step-scale getter (60ee4367.js).
    pub fn scale(self) -> u32 {
        match self {
            Precision::P001 => 100,
            Precision::P0005 => 200,
            Precision::P0001 => 1000,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Precision::P001 => "0.01mm",
            Precision::P0005 => "0.005mm",
            Precision::P0001 => "0.001mm",
        }
    }
}

/// Fallback scale from firmware USB/RF version (7a5b12c9.js):
/// `None=>10, 768<=v<1280=>100, >=1280=>200, else 10`.
pub fn scale_from_version(usb: Option<u16>, rf: Option<u16>) -> u32 {
    // JS `get磁轴行程步进倍数`: `const n = rf || usb` — a zero RF version
    // falls back to USB (unlike the calibration threshold's `rf ?? usb`).
    let v = match rf {
        Some(n) if n != 0 => Some(n),
        _ => usb.filter(|n| *n != 0),
    };
    match v {
        None => 10,
        Some(n) if n >= 768 && n < 1280 => 100,
        Some(n) if n >= 1280 => 200,
        _ => 10,
    }
}

/// Per-key travel-calibration success threshold from the original UI
/// (`mj` in `index.5627ed60.js`):
/// `const y = version.rf ?? version.usb; const f = y && y < 768 ? 1 : 300;`.
/// Versions below 768 report a different raw scale, so the driver uses 1;
/// modern firmware uses 300 raw units (3.00 mm at scale 100).
pub fn calibration_threshold(usb: Option<u16>, rf: Option<u16>) -> u16 {
    let v = rf.or(usb).unwrap_or(0);
    if v != 0 && v < 768 {
        1
    } else {
        300
    }
}

/// Checksum: `Wi = sum(bytes)`, `r[7 or 8] = 255 - (sum & 255)`.
/// JS: `Wi=e=>e.reduce((t,i)=>t+i,0)` + `___encodeCmd`.
pub fn checksum(bytes: &[u8]) -> u8 {
    let sum: u32 = bytes.iter().map(|b| *b as u32).sum();
    255u8.wrapping_sub((sum & 0xFF) as u8)
}

/// Encode an outgoing command exactly like JS `___encodeCmd(t,i)`:
/// pad to 9 bytes if shorter, then write checksum at [7] (Bit7) or [8] (Bit8).
/// Longer buffers (e.g. 64 B keyboard commands) keep their length;
/// only the checksum byte is overwritten. `None` leaves the buffer untouched.
pub fn encode_cmd(mut buf: Vec<u8>, kind: ReportKind) -> Vec<u8> {
    if buf.len() < 9 {
        buf.resize(9, 0);
    }
    match kind {
        ReportKind::Bit7 => {
            let s = checksum(&buf[0..7]);
            buf[7] = s;
        }
        ReportKind::Bit8 => {
            let s = checksum(&buf[0..8]);
            buf[8] = s;
        }
        ReportKind::None => {}
    }
    buf
}

/// Build `GET_MULTI_MAGNETISM` request: `[229, op, 1, page, 0,0,0,0]` + Bit7 checksum.
/// JS `_getMulitMagnetismCMD(t,e)`: `s[0]=229,s[1]=t,s[2]=1,s[3]=page`.
pub fn build_get_multi_magnetism(op: u8, page: u8) -> Vec<u8> {
    encode_cmd(
        vec![cmd::GET_MULTI_MAGNETISM, op, 1, page, 0, 0, 0, 0],
        ReportKind::Bit7,
    )
}

/// Build the original single-slot magnetism write:
/// `[101, op, 0, slot, last, 0, 0, checksum]` + payload bytes.
/// JS `_sendMagnetismInfoSimpleCMD(op, slot, last, data)` appends the
/// payload after the fixed 8-byte header; travel values are two bytes
/// `[raw & 255, raw >> 8]` and `last=1` applies the change.
pub fn build_magnetism_simple(op: u8, slot: u8, last: bool, data: &[u8]) -> Vec<u8> {
    let mut buf = vec![
        cmd::SET_MULTI_MAGNETISM,
        op,
        0,
        slot,
        u8::from(last),
        0,
        0,
        0,
    ];
    buf.extend_from_slice(data);
    encode_cmd(buf, ReportKind::Bit7)
}

/// Build the ordered single-slot write batch used by
/// `setMagnetismInfoSimple`: every op except the final one gets `last=0`,
/// the final op gets `last=1` (the only point where the firmware applies
/// the queued changes).
pub fn build_magnetism_batch(slot: u8, ops: &[(u8, u16)]) -> Vec<Vec<u8>> {
    ops.iter()
        .enumerate()
        .map(|(i, (op, raw))| {
            build_magnetism_simple(*op, slot, i + 1 == ops.len(), &raw.to_le_bytes())
        })
        .collect()
}

/// Decode LE u16 list, JS `_decodeMultiMagnetismTwoByteList`.
pub fn decode_u16le_list(bytes: &[u8]) -> Vec<u16> {
    bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect()
}

/// Convert raw travel units to mm with the given scale.
pub fn raw_to_mm(raw: u16, scale: u32) -> f32 {
    raw as f32 / scale as f32
}

/// Display decimals for travel mm values — the driver's `travelPrecision`
/// getter (`index.5627ed60.js`): number of decimal digits of the
/// press-travel step, default 2. With step = 1/scale that is 2 for
/// scale 100 (0.01 mm), 3 for 200 (0.005 mm) / 1000 (0.001 mm).
pub fn travel_digits(scale: u32) -> usize {
    let mut d = 0usize;
    let mut v = 1u32;
    while v < scale && d < 9 {
        v *= 10;
        d += 1;
    }
    d.clamp(1, 9)
}

/// Parse `GET_FEATURE_LIST (230)` response: must be `[230, 0xAA, precision, ...]`.
pub fn parse_feature_list(resp: &[u8]) -> Option<Precision> {
    if resp.len() < 3 || resp[0] != cmd::GET_FEATURE_LIST || resp[1] != 0xAA {
        return None;
    }
    match resp[2] {
        0 => Some(Precision::P001),
        1 => Some(Precision::P0005),
        2 => Some(Precision::P0001),
        _ => None,
    }
}

/// Parse `GET_USB_VERSION (143)` keyboard response: `version = r[8]<<8 | r[7]`.
pub fn parse_usb_version(resp: &[u8]) -> Option<u16> {
    if resp.len() < 9 || resp[0] != cmd::GET_USB_VERSION {
        return None;
    }
    Some(((resp[8] as u16) << 8) | resp[7] as u16)
}

/// Parse `whoAmI` (also cmd 143): `id = u32 LE at bytes 1..4`, 0xFFFFFFFF = none.
pub fn parse_device_id(resp: &[u8]) -> Option<u32> {
    if resp.len() < 5 || resp[0] != cmd::GET_USB_VERSION {
        return None;
    }
    let id = u32::from_le_bytes([resp[1], resp[2], resp[3], resp[4]]);
    if id == 0xFFFF_FFFF {
        None
    } else {
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_matches_js_example() {
        // JS: r[7] = 255 - (sum(r[0..7]) & 255)
        let buf = vec![27u8, 1, 0, 0, 0, 0, 0, 0]; // SET_MAGNETISM_REPOR enable
        let enc = encode_cmd(buf, ReportKind::Bit7);
        assert_eq!(enc[7], 255 - ((27 + 1) & 0xFF) as u8);
    }

    #[test]
    fn bit8_checksum_covers_8_bytes() {
        let buf = vec![7u8, 1, 2, 3, 4, 5, 6, 7, 0];
        let enc = encode_cmd(buf.clone(), ReportKind::Bit8);
        let expect = 255 - ((7 + 1 + 2 + 3 + 4 + 5 + 6 + 7) & 0xFF) as u8;
        assert_eq!(enc[8], expect);
    }

    #[test]
    fn get_multi_magnetism_request_layout() {
        let r = build_get_multi_magnetism(mag_op::LIVE_TRAVEL, 2);
        assert_eq!(&r[0..4], &[229, 254, 1, 2]);
        assert_eq!(r[7], checksum(&r[0..7]));
    }

    #[test]
    fn decode_u16le_matches_js() {
        assert_eq!(decode_u16le_list(&[0x64, 0x00, 0xC8, 0x00]), vec![100, 200]);
        assert_eq!(raw_to_mm(200, 100), 2.0);
        assert!((raw_to_mm(5, 1000) - 0.005).abs() < 1e-6);
    }

    #[test]
    fn travel_digits_match_driver_precision() {
        // travelPrecision = decimals of the step 1/scale, default 2.
        assert_eq!(travel_digits(100), 2); // 0.01 mm
        assert_eq!(travel_digits(200), 3); // 0.005 mm
        assert_eq!(travel_digits(1000), 3); // 0.001 mm
        assert_eq!(travel_digits(10), 1); // 0.1 mm
    }

    #[test]
    fn press_lift_travel_request_layout() {
        // getMultiMagnetismInfo: _getMulitMagnetismCMD(0,4) / (1,4).
        let p = build_get_multi_magnetism(mag_op::TRAVEL, 4);
        assert_eq!(&p[0..4], &[229, 0, 1, 4]);
        let l = build_get_multi_magnetism(mag_op::LIFT_TRAVEL, 4);
        assert_eq!(&l[0..4], &[229, 1, 1, 4]);
        let d = build_get_multi_magnetism(mag_op::DEADZONE, 4);
        assert_eq!(&d[0..4], &[229, 6, 1, 4]);
    }

    #[test]
    fn travel_write_layout_matches_simple_command() {
        // _sendMagnetismInfoSimpleCMD(0, slot, last, [lo, hi]):
        // [101,0,0,slot,last,0,0,checksum,lo,hi].
        let press = build_magnetism_simple(mag_op::TRAVEL, 41, false, &200u16.to_le_bytes());
        assert_eq!(press.len(), 10);
        assert_eq!(&press[0..5], &[101, 0, 0, 41, 0]);
        assert_eq!(&press[8..10], &[200, 0]);
        assert_eq!(press[7], checksum(&press[0..7]));

        let lift = build_magnetism_simple(mag_op::LIFT_TRAVEL, 41, true, &280u16.to_le_bytes());
        assert_eq!(&lift[0..5], &[101, 1, 0, 41, 1]);
        assert_eq!(&lift[8..10], &[24, 1]); // 280 = 0x0118
        assert_eq!(lift[7], checksum(&lift[0..7]));
    }

    #[test]
    fn magnetism_batch_applies_only_on_the_final_changed_op() {
        let batch = build_magnetism_batch(
            41,
            &[
                (mag_op::TRAVEL, 220),
                (mag_op::LIFT_TRAVEL, 240),
                (mag_op::DEADZONE, 40),
            ],
        );
        assert_eq!(batch.len(), 3);
        assert_eq!(&batch[0][0..5], &[101, 0, 0, 41, 0]);
        assert_eq!(&batch[1][0..5], &[101, 1, 0, 41, 0]);
        assert_eq!(&batch[2][0..5], &[101, 6, 0, 41, 1]);

        // A single changed op is also the final applying op.
        let batch = build_magnetism_batch(41, &[(mag_op::LIFT_TRAVEL, 200)]);
        assert_eq!(&batch[0][0..5], &[101, 1, 0, 41, 1]);
    }

    #[test]
    fn deadzone_write_layout_matches_simple_command() {
        // setMagnetismInfoSimple op 6: deadZoneTravel * scale as u16 LE.
        let deadzone = build_magnetism_simple(mag_op::DEADZONE, 41, true, &30u16.to_le_bytes());
        assert_eq!(deadzone.len(), 10);
        assert_eq!(&deadzone[0..5], &[101, 6, 0, 41, 1]);
        assert_eq!(&deadzone[8..10], &[30, 0]);
        assert_eq!(deadzone[7], checksum(&deadzone[0..7]));
    }

    #[test]
    fn travel_defaults_and_limits_match_original() {
        assert!((travel_max_mm(Some(768), None) - 3.3).abs() < 1e-6);
        assert!((travel_max_mm(Some(1024), Some(900)) - 3.3).abs() < 1e-6);
        assert!((travel_max_mm(Some(500), None) - 4.0).abs() < 1e-6);
        assert!((travel_max_mm(Some(1024), Some(500)) - 4.0).abs() < 1e-6);
        assert!((travel_max_mm(Some(0), None) - 3.3).abs() < 1e-6);
        assert!((deadzone_max_mm(Some(768), None) - 1.0).abs() < 1e-6);
        assert!((deadzone_max_mm(Some(500), None) - 4.0).abs() < 1e-6);
        assert!((deadzone_max_mm(Some(1024), Some(500)) - 4.0).abs() < 1e-6);
        assert!((deadzone_max_mm(Some(0), None) - 1.0).abs() < 1e-6);
        assert_eq!(mm_to_raw(TRAVEL_MIN_MM, 100), 10);
        assert_eq!(mm_to_raw(TRAVEL_DEFAULT_PRESS_MM, 100), 200);
        assert_eq!(mm_to_raw(TRAVEL_DEFAULT_LIFT_MM, 100), 280);
        assert_eq!(mm_to_raw(TRAVEL_DEFAULT_DEADZONE_MM, 100), 30);
        assert_eq!(mm_to_raw(TRAVEL_MAX_DEADZONE_LEGACY_MM, 100), 400);
    }

    #[test]
    fn mm_to_raw_truncates_like_js_bitwise_ops() {
        // JS `(v * scale) & 255` truncates toward zero, not rounds.
        assert_eq!(mm_to_raw(0.29, 100), 28);
        assert_eq!(mm_to_raw(1.15, 100), 114);
        assert_eq!(mm_to_raw(2.01, 100), 200);
        assert_eq!(mm_to_raw(2.29, 100), 229);
        // Decimal constants keep their exact integer raw value.
        assert_eq!(mm_to_raw(2.8, 100), 280);
        assert_eq!(mm_to_raw(3.3, 100), 330);
        assert_eq!(mm_to_raw(2.8, 200), 560);
        assert_eq!(mm_to_raw(2.8, 1000), 2800);
    }

    #[test]
    fn feature_list_parsing() {
        assert_eq!(parse_feature_list(&[230, 0xAA, 0]), Some(Precision::P001));
        assert_eq!(parse_feature_list(&[230, 0xAA, 1]), Some(Precision::P0005));
        assert_eq!(parse_feature_list(&[230, 0xAA, 2]), Some(Precision::P0001));
        assert_eq!(parse_feature_list(&[230, 0x00, 0]), None);
    }

    #[test]
    fn scale_fallback_matches_js() {
        assert_eq!(scale_from_version(None, None), 10);
        assert_eq!(scale_from_version(Some(800), None), 100);
        assert_eq!(scale_from_version(Some(1300), None), 200);
        assert_eq!(scale_from_version(Some(500), None), 10);
        // `rf || usb`: zero RF falls back to USB.
        assert_eq!(scale_from_version(Some(800), Some(0)), 100);
        assert_eq!(scale_from_version(Some(0), Some(1300)), 200);
        assert_eq!(Precision::P001.scale(), 100);
        assert_eq!(Precision::P0005.scale(), 200);
        assert_eq!(Precision::P0001.scale(), 1000);
    }

    #[test]
    fn calibration_threshold_matches_driver() {
        // JS `y && y < 768 ? 1 : 300`, with rf preferred over usb.
        assert_eq!(calibration_threshold(None, None), 300);
        assert_eq!(calibration_threshold(Some(0), None), 300);
        assert_eq!(calibration_threshold(Some(1), None), 1);
        assert_eq!(calibration_threshold(Some(767), None), 1);
        assert_eq!(calibration_threshold(Some(768), None), 300);
        assert_eq!(calibration_threshold(Some(1024), None), 300);
        assert_eq!(calibration_threshold(Some(1024), Some(512)), 1);
        assert_eq!(calibration_threshold(Some(512), Some(0)), 300);
    }

    #[test]
    fn usb_version_and_id_offsets() {
        let mut r = vec![0u8; 64];
        r[0] = 143;
        r[1] = 0x34;
        r[2] = 0x12;
        r[3] = 0x00;
        r[4] = 0x00;
        r[7] = 0x00;
        r[8] = 0x04; // version 0x0400 = 1024
        assert_eq!(parse_usb_version(&r), Some(0x0400));
        assert_eq!(parse_device_id(&r), Some(0x1234));
    }
}
