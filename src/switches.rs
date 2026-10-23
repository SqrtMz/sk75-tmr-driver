//! Switch (axis) type selection for SK75 TMR.
//!
//! The original driver lets the user pick the installed magnetic switch
//! per key ("replace axis" flow: choose the key, then the axis option).
//! Wire format (`dist/js/7a5b12c9.js`): axis code table `P` in the index
//! bundle (`Gaote=0, …`), single-key write
//! `_sendMagnetismInfoSimpleCMD(252, slot, true, [encode(name)])`
//! i.e. 8-byte header `[101,252,0,slot,1,0,0,checksum]` plus the axis
//! code at byte 8; batch read `_getMulitMagnetismCMD(252,2)` (one byte
//! per matrix slot).
//! REA: `analyze_javascript_application` summary `ev_927021fe…`
//! + module page `ev_6b3387ab…`. The code-at-byte-8 layout was
//! re-confirmed with REA (parent `ev_f2e38f5e…`, summary
//! `ev_270e853a…`); the earlier 6-byte request made the firmware store 0.
//!
//! SK75 device entry (`index.5627ed60.js`, id 2518 + 3804):
//! replaceable axes, six supported types, with a special display name
//! mapping POM magnetic-king -> "POM Magnetic-King".

use anyhow::Result;

use crate::hid::Sk75;

/// Supported SK75 axes: (wire code, display label).
///
/// Display = "{original-driver English name} - {name in source}", per
/// request. Driver-side English names come from the English i18n bundle
/// `dist/js/9528bb70.js` (`高特:"Outemu"`, `磁玉pro:"Magnetic Jade Pro"`,
/// `磁玉gaming:"Magnetic Jade Gaming"`, `万磁王RGB:"Magneto RGB"`,
/// `万磁王POM:"POM Magnet King"`, `玄磁轴:"Void Switch"`) plus the SK75
/// device entry's `specialSwitchDisplayName:{万磁王POM:"POM万磁王"}` which
/// maps the table name 万磁王POM to the `POM万磁王` i18n key before lookup
/// (REA `ev_a6632e7b…`, scope includes `9528bb70.js`+`index.5627ed60.js`).
pub const SK75_SWITCHES: [(u8, &str); 6] = [
    (0, "Outemu - Gaote"),
    (2, "Magnetic Jade Pro - Jade Pro"),
    (3, "Magnetic Jade Gaming - Jade Gaming"),
    (12, "Magneto RGB - Magnet-King RGB"),
    (13, "POM Magnet King - POM Magnet-King"),
    (69, "Void Switch - Xuan Magnetic"),
];

/// Per-switch key colors for the keyboard preview, straight from the
/// driver's switch-color dictionary `轴体颜色字典` (`index.5627ed60.js`):
/// `高特:16771625, 磁玉pro:3080153, 磁玉gaming:12026111,
/// 万磁王RGB:15746815, 万磁王POM:14835949, 玄磁轴:6970061` (decimal RGB,
/// i.e. `0xRRGGBB`). The driver fills each key's switch-shape path with
/// `轴体颜色字典[轴体] ?? 16771625` and falls back to axis 高特
/// (`轴体||"高特"`) for unknown codes, so unset/unknown slots get the
/// Outemu color and switch number 1, exactly like the original.
pub const SK75_SWITCH_COLORS: [[u8; 3]; 6] = [
    [255, 234, 41],  // 高特 16771625 -> #FFEA29
    [46, 255, 217],  // 磁玉pro 3080153 -> #2EFFD9
    [183, 128, 255], // 磁玉gaming 12026111 -> #B780FF
    [240, 70, 255],  // 万磁王RGB 15746815 -> #F046FF
    [226, 96, 237],  // 万磁王POM 14835949 -> #E260ED
    [106, 90, 205],  // 玄磁轴 6970061 -> #6A5ACD
];

/// Color for a raw axis code (driver fallback rules included).
pub fn switch_color(code: u8) -> [u8; 3] {
    match SK75_SWITCHES.iter().position(|(c, _)| *c == code) {
        Some(i) => SK75_SWITCH_COLORS[i],
        // `轴体颜色字典[轴体]||16771625` + `_decode` falls back to 高特.
        _ => SK75_SWITCH_COLORS[0],
    }
}

/// 1-based switch number shown in the driver preview
/// (`supportedSwitchTypes.findIndex(轴体)+1`; unknown/0xFF decodes as
/// 高特 -> 1).
#[cfg(test)]
pub fn switch_number(code: u8) -> u8 {
    SK75_SWITCHES
        .iter()
        .position(|(c, _)| *c == code)
        .map(|i| i as u8 + 1)
        .unwrap_or(1)
}

pub fn switch_label(code: u8) -> String {
    SK75_SWITCHES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, l)| l.to_string())
        .unwrap_or_else(|| format!("Unknown axis ({code})"))
}

/// Display label for a raw code. The driver falls back to Gaote for
/// unknown codes (`_decode` returns "Gaote"); unwritten slots read back
/// 0xFF, shown here explicitly instead of silently relabeled.
pub fn switch_display(code: u8) -> String {
    if code == 0xFF {
        return String::from("Unset (driver default Gaote)");
    }
    switch_label(code)
}

#[cfg(test)]
pub fn switch_code(label: &str) -> Option<u8> {
    SK75_SWITCHES
        .iter()
        .find(|(_, l)| *l == label)
        .map(|(c, _)| *c)
}

/// Build the single-key axis write request (before Bit7 checksum encoding):
/// 8-byte header `[101,252,0,slot,1,0,0,0]` + code at byte 8. Mirrors JS
/// `_sendMagnetismInfoSimpleCMD(t,e,n,i)`, which appends its data argument
/// after the fixed 8-byte header.
pub fn build_switch_write(slot: u8, code: u8) -> Vec<u8> {
    vec![101, 252, 0, slot, 1, 0, 0, 0, code]
}

impl Sk75 {
    /// Batch-read per-slot axis codes (op 252, 2 pages = 128 bytes).
    pub fn read_switch_types_raw(&self) -> Result<Vec<u8>> {
        let blob = self.get_multi_magnetism_with_timing(
            crate::proto::mag_op::AXIS_TYPE,
            2,
            crate::hid::SEND_DELAY,
            crate::hid::READ_DELAY,
        )?;
        Ok(blob.into_iter().take(128).collect())
    }

    /// Single-key axis write, matching JS `_sendMagnetismInfoSimpleCMD`:
    /// 8-byte header `[101,252,0,slot,1,0,0,checksum]` with the axis code
    /// appended at index 8 (`[101,252,0,slot,1,0,0,checksum,code]`).
    /// The code must be at byte 8; placing it at byte 5 makes the firmware
    /// read 0 and store Outemu/Gaote for every write.
    pub fn write_switch_type(&self, slot: usize, code: u8) -> Result<()> {
        if slot > 127 {
            anyhow::bail!("slot {slot} out of range (0..127)");
        }
        let req = build_switch_write(slot as u8, code);
        self.send_msg_with_timing(&req, crate::proto::ReportKind::Bit7, crate::hid::SEND_DELAY)?;
        Self::vendor_sleep(None);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sk75_table_has_six_entries() {
        assert_eq!(SK75_SWITCHES.len(), 6);
        // Hyphenated display names: original driver name - source name.
        assert_eq!(switch_label(0), "Outemu - Gaote");
        assert_eq!(switch_label(13), "POM Magnet King - POM Magnet-King");
        assert_eq!(switch_code("Void Switch - Xuan Magnetic"), Some(69));
        assert_eq!(switch_code("Magnetic Jade Gaming - Jade Gaming"), Some(3));
    }

    #[test]
    fn switch_colors_match_driver_dictionary() {
        // Decimal values from 轴体颜色字典 in index.5627ed60.js.
        assert_eq!(switch_color(0), [255, 234, 41]); // 高特 16771625
        assert_eq!(switch_color(2), [46, 255, 217]); // 磁玉pro 3080153
        assert_eq!(switch_color(3), [183, 128, 255]); // 磁玉gaming 12026111
        assert_eq!(switch_color(12), [240, 70, 255]); // 万磁王RGB 15746815
        assert_eq!(switch_color(13), [226, 96, 237]); // 万磁王POM 14835949
        assert_eq!(switch_color(69), [106, 90, 205]); // 玄磁轴 6970061
                                                      // Driver fallback: unknown code/unwritten slot -> 高特 color, number 1.
        assert_eq!(switch_color(0xFF), [255, 234, 41]);
        assert_eq!(switch_color(7), [255, 234, 41]);
        assert_eq!(switch_number(0xFF), 1);
        assert_eq!(switch_number(69), 6);
    }

    #[test]
    fn single_write_layout_matches_driver() {
        // _sendMagnetismInfoSimpleCMD(252, slot, true, [code]):
        // [101,252,0,slot,1,0,0,checksum] + code at index 8.
        let req = build_switch_write(41, 13);
        assert_eq!(req, vec![101, 252, 0, 41, 1, 0, 0, 0, 13]);
        let enc = crate::proto::encode_cmd(req, crate::proto::ReportKind::Bit7);
        assert_eq!(&enc[0..5], &[101, 252, 0, 41, 1]);
        assert_eq!(enc[8], 13);
        assert_eq!(enc[7], crate::proto::checksum(&enc[0..7]));
    }
}
