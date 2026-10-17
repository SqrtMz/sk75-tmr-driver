//! Consolidated keyboard-information readout for UI + CLI.
//!
//! Mirrors the original driver's device-info panel
//! (`dist/js/f70eb088.js`: white card rows firmware-version + firmware-upgrade
//! button and IOT-version + check-update button) and the underlying
//! queries in `dist/js/7a5b12c9.js` (`getUSBVersion`, `getRFVersion`,
//! `getReportRate`, `whoAmI` via cmd 143) and `dist/js/60ee4367.js`
//! (`getPrecisionEnum` via FEA 230). REA: `analyze_javascript_application`
//! summary `ev_a9d5046b…` (5 scoped modules) + module page
//! `ev_43d23e55…` / item `ev_ccc7a2e4…`.

use crate::{hid, matrix, proto};

/// Everything the UI's "keyboard information" option shows.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyboardInfo {
    pub display_name: String,
    pub vid: u16,
    pub pid: u16,
    pub path: String,
    pub manufacturer: String,
    pub product: String,
    pub device_id: Option<u32>,
    pub variant: String,
    pub usb_version: Option<u16>,
    pub precision_label: Option<String>,
    pub scale: u32,
    pub report_rate_hz: Option<u32>,
    pub key_count: usize,
    pub error: Option<String>,
}

impl KeyboardInfo {
    pub fn offline(product: &str, vid: u16, pid: u16) -> Self {
        Self {
            display_name: product.to_string(),
            vid,
            pid,
            path: String::from("<no device>"),
            manufacturer: String::new(),
            product: product.to_string(),
            device_id: None,
            variant: String::from("unknown (offline)"),
            usb_version: None,
            precision_label: None,
            scale: 100,
            report_rate_hz: None,
            key_count: matrix::populated_keys(&matrix::MATRIX_2518).len(),
            error: Some(String::from("no readable device; showing static layout")),
        }
    }

    pub fn vid_pid_hex(&self) -> String {
        format!("{:04x}:{:04x}", self.vid, self.pid)
    }

    pub fn vid_pid_dec(&self) -> String {
        format!("{}:{}", self.vid, self.pid)
    }

    pub fn device_id_text(&self) -> String {
        match self.device_id {
            Some(2518) => String::from("2518 (ry5088_womiersk75he_8k_8k)"),
            Some(3804) => String::from("3804 (ry5088_womier_sk75he_europe_3m_8k_8k)"),
            Some(o) => format!("{o} (unknown variant)"),
            None => String::from("unreadable"),
        }
    }

    pub fn usb_version_text(&self) -> String {
        match self.usb_version {
            Some(v) => format!("{v} (0x{v:04x})"),
            None => String::from("unreadable"),
        }
    }

    pub fn precision_text(&self) -> String {
        match &self.precision_label {
            Some(p) => format!("{p} (scale {})", self.scale),
            None => format!("unknown, fallback scale {}", self.scale),
        }
    }

    pub fn report_rate_text(&self) -> String {
        match self.report_rate_hz {
            Some(hz) => format!("{hz} Hz"),
            None => String::from("unreadable"),
        }
    }
}

fn variant_of(device_id: Option<u32>) -> (String, &'static [u8; 512]) {
    match device_id {
        Some(3804) => (
            String::from("SK75 TMR Europe 3-mode (3804)"),
            &matrix::MATRIX_3804,
        ),
        Some(2518) => (
            String::from("SK75 TMR (2518, ry5088_womiersk75he_8k_8k)"),
            &matrix::MATRIX_2518,
        ),
        Some(o) => (
            format!("SK75 TMR variant {o} (generic 81-key layout)"),
            &matrix::MATRIX_2518,
        ),
        None => (
            String::from("SK75 TMR (default 2518 layout)"),
            &matrix::MATRIX_2518,
        ),
    }
}

/// Open the vendor interface and read all info fields.
/// Never fails outright: transport errors are captured in `error`
/// (e.g. hidraw permission denied) so the UI can display them with
/// the udev/wired-mode hint instead of crashing.
pub fn read_keyboard_info(vid: u16, pid: u16, path: Option<&str>, iface: Option<i32>) -> KeyboardInfo {
    let dev = match hid::open_sk75(vid, pid, path, iface) {
        Ok(d) => d,
        Err(e) => {
            let mut info = KeyboardInfo::offline("SK75 TMR", vid, pid);
            info.error = Some(format!("{e:#}. Hint: wired mode + udev rule udev/99-sk75-tmr.rules"));
            return info;
        }
    };
    let meta = hid::enumerate_sk75(vid, pid)
        .ok()
        .and_then(|v| {
            v.into_iter()
                .find(|d| d.path().to_string_lossy() == dev.path)
        });
    let device_id = dev.device_id().unwrap_or(None);
    let usb_version = dev.usb_version().unwrap_or(None);
    let precision = dev.precision().unwrap_or(None);
    let scale = dev
        .travel_scale()
        .unwrap_or_else(|_| {
            proto::scale_from_version(usb_version.filter(|v| *v != 0), None)
        });
    // Report-rate is best-effort; older firmware may not answer.
    let report_rate_hz = dev.report_rate_hz().unwrap_or(None);
    let (variant, mat) = variant_of(device_id);
    let mut error = None;
    if device_id.is_none() && usb_version.is_none() && precision.is_none() {
        error = Some(String::from(
            "device opened butqueries failed; try wired mode and re-plug",
        ));
    }
    KeyboardInfo {
        display_name: String::from("SK75 TMR"),
        vid,
        pid,
        path: dev.path.clone(),
        manufacturer: meta
            .as_ref()
            .and_then(|d| d.manufacturer_string())
            .unwrap_or("")
            .trim()
            .to_string(),
        product: meta
            .as_ref()
            .and_then(|d| d.product_string())
            .unwrap_or("SK75 TMR")
            .trim()
            .to_string(),
        device_id,
        variant,
        usb_version,
        precision_label: precision.map(|p| p.label().to_string()),
        scale,
        report_rate_hz,
        key_count: matrix::populated_keys(mat).len(),
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_info_has_layout_key_count() {
        let info = KeyboardInfo::offline("SK75 TMR", 0x3151, 0x5030);
        assert!(info.key_count >= 80 && info.key_count <= 90);
        assert_eq!(info.vid_pid_hex(), "3151:5030");
        assert_eq!(info.device_id_text(), "unreadable");
    }

    #[test]
    fn variant_labels_match_rea_device_db() {
        assert!(variant_of(Some(2518)).0.contains("2518"));
        assert!(variant_of(Some(3804)).0.contains("3804"));
        assert_eq!(variant_of(Some(3804)).1.len(), 512);
    }

    #[test]
    fn unreadable_device_returns_hint_not_panic() {
        // No such VID:PID on test hosts; must return error text, not Err.
        let info = read_keyboard_info(0x0000, 0x0000, None, None);
        assert!(info.error.is_some());
        assert_eq!(info.display_name, "SK75 TMR");
    }
}
