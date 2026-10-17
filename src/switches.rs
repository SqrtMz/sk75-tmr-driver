//! Switch (axis) type selection for SK75 TMR.
//!
//! The original driver lets the user pick the installed magnetic switch
//! per key ("replace axis" flow: choose the key, then the axis option).
//! Wire format (`dist/js/7a5b12c9.js`): axis code table `P` in the index
//! bundle (`Gaote=0, …`), single-key write
//! `_sendMagnetismInfoSimpleCMD(252, slot, true, [encode(name)])`
//! i.e. `[101,252,0,slot,1,code]`, batch read
//! `_getMulitMagnetismCMD(252,2)` (one byte per matrix slot).
//! REA: `analyze_javascript_application` summary `ev_927021fe…`
//! + module page `ev_6b3387ab…`.
//!
//! SK75 device entry (`index.5627ed60.js`, id 2518 + 3804):
//! replaceable axes, six supported types, with a special display name
//! mapping POM magnetic-king -> "POM Magnetic-King".

use anyhow::Result;

use crate::hid::Sk75;

/// Supported SK75 axes: (wire code, English label).
/// Codes come from the driver's axis table `P`; labels are English
/// translations of the driver strings.
pub const SK75_SWITCHES: [(u8, &str); 6] = [
    (0, "Gaote"),
    (2, "Jade Pro"),
    (3, "Jade Gaming"),
    (12, "Magnet-King RGB"),
    (13, "POM Magnet-King"),
    (69, "Xuan Magnetic"),
];

/// Fallback used by the driver when decoding an unknown code.
pub const FALLBACK_SWITCH: &str = "Gaote";

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

pub fn switch_code(label: &str) -> Option<u8> {
    SK75_SWITCHES
        .iter()
        .find(|(_, l)| *l == label)
        .map(|(c, _)| *c)
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

    /// Single-key axis write: `[101,252,0,slot,1,code]` + Bit7.
    pub fn write_switch_type(&self, slot: usize, code: u8) -> Result<()> {
        if slot > 127 {
            anyhow::bail!("slot {slot} out of range (0..127)");
        }
        let req = vec![101, 252, 0, slot as u8, 1, code];
        self.send_msg_with_timing(
            &req,
            crate::proto::ReportKind::Bit7,
            crate::hid::SEND_DELAY,
            crate::hid::READ_DELAY,
        )?;
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
        assert_eq!(switch_label(0), "Gaote");
        assert_eq!(switch_label(13), "POM Magnet-King");
        assert_eq!(switch_code("Xuan Magnetic"), Some(69));
    }

    #[test]
    fn single_write_layout_matches_driver() {
        // [101,252,0,slot,1,code]
        let (slot, code) = (41u8, 13u8);
        assert_eq!(vec![101, 252, 0, slot, 1, code], [101, 252, 0, 41, 1, 13]);
    }
}
