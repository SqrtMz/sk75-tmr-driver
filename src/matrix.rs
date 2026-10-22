//! SK75 TMR key matrix + HID usage names.
//!
//! Matrices from `dist/js/7de29f04.js` (id 2518, `ry5088_womiersk75he_8k_8k`)
//! and `dist/js/74d1dc6b.js` (id 3804, `ry5088_womier_sk75he_europe_3m_8k_8k`).
//! Each is 512 u8 = 128 slots x 4 B. Non-zero slots are real keys;
//! `[0,0,hid,0]` decodes via JS `matrixToHid` to the USB HID usage `hid`.
//! The two variants differ only at slots 65/67 (`[55,0,10,1]` marker moves).
//! Fn-layer defaults (`FN_MATRIX_*`) come from the same modules'
//! `defaultFnMatrix`/`defaultFnMacMatrix` tables (512 u8 each).

/// Matrix for id 2518 (`ry5088_womiersk75he_8k_8k`).
pub const MATRIX_2518: [u8; 512] = [
    0, 0, 41, 0, 0, 0, 53, 0, 0, 0, 43, 0, 0, 0, 57, 0, 0, 0, 225, 0, 0, 0, 224, 0, 0, 0, 58, 0, 0,
    0, 30, 0, 0, 0, 20, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 59, 0, 0, 0, 31, 0, 0, 0, 26,
    0, 0, 0, 22, 0, 0, 0, 29, 0, 0, 0, 227, 0, 0, 0, 60, 0, 0, 0, 32, 0, 0, 0, 8, 0, 0, 0, 7, 0, 0,
    0, 27, 0, 0, 0, 226, 0, 0, 0, 61, 0, 0, 0, 33, 0, 0, 0, 21, 0, 0, 0, 9, 0, 0, 0, 6, 0, 0, 0, 0,
    0, 0, 0, 62, 0, 0, 0, 34, 0, 0, 0, 23, 0, 0, 0, 10, 0, 0, 0, 25, 0, 0, 0, 0, 0, 0, 0, 63, 0, 0,
    0, 35, 0, 0, 0, 28, 0, 0, 0, 11, 0, 0, 0, 5, 0, 0, 0, 44, 0, 0, 0, 64, 0, 0, 0, 36, 0, 0, 0,
    24, 0, 0, 0, 13, 0, 0, 0, 17, 0, 0, 0, 0, 0, 0, 0, 65, 0, 0, 0, 37, 0, 0, 0, 12, 0, 0, 0, 14,
    0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 66, 0, 0, 0, 38, 0, 0, 0, 18, 0, 0, 0, 15, 0, 0, 0, 54, 0, 0,
    0, 0, 0, 0, 0, 67, 0, 0, 0, 39, 0, 0, 0, 19, 0, 0, 0, 51, 0, 0, 0, 55, 0, 10, 1, 0, 0, 0, 0,
    68, 0, 0, 0, 45, 0, 0, 0, 47, 0, 0, 0, 52, 0, 0, 0, 56, 0, 0, 0, 228, 0, 0, 0, 69, 0, 0, 0, 46,
    0, 0, 0, 48, 0, 0, 0, 0, 0, 0, 0, 229, 0, 0, 0, 80, 0, 0, 0, 76, 0, 0, 0, 42, 0, 0, 0, 49, 0,
    0, 0, 40, 0, 0, 0, 82, 0, 0, 0, 81, 0, 0, 0, 74, 0, 0, 0, 77, 0, 0, 0, 75, 0, 0, 0, 78, 0, 0,
    0, 0, 0, 0, 0, 79, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// Matrix for id 3804 (`ry5088_womier_sk75he_europe_3m_8k_8k`, Europe 3-mode).
pub const MATRIX_3804: [u8; 512] = [
    0, 0, 41, 0, 0, 0, 53, 0, 0, 0, 43, 0, 0, 0, 57, 0, 0, 0, 225, 0, 0, 0, 224, 0, 0, 0, 58, 0, 0,
    0, 30, 0, 0, 0, 20, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 59, 0, 0, 0, 31, 0, 0, 0, 26,
    0, 0, 0, 22, 0, 0, 0, 29, 0, 0, 0, 227, 0, 0, 0, 60, 0, 0, 0, 32, 0, 0, 0, 8, 0, 0, 0, 7, 0, 0,
    0, 27, 0, 0, 0, 226, 0, 0, 0, 61, 0, 0, 0, 33, 0, 0, 0, 21, 0, 0, 0, 9, 0, 0, 0, 6, 0, 0, 0, 0,
    0, 0, 0, 62, 0, 0, 0, 34, 0, 0, 0, 23, 0, 0, 0, 10, 0, 0, 0, 25, 0, 0, 0, 0, 0, 0, 0, 63, 0, 0,
    0, 35, 0, 0, 0, 28, 0, 0, 0, 11, 0, 0, 0, 5, 0, 0, 0, 44, 0, 0, 0, 64, 0, 0, 0, 36, 0, 0, 0,
    24, 0, 0, 0, 13, 0, 0, 0, 17, 0, 0, 0, 0, 0, 0, 0, 65, 0, 0, 0, 37, 0, 0, 0, 12, 0, 0, 0, 14,
    0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 66, 0, 0, 0, 38, 0, 0, 0, 18, 0, 0, 0, 15, 0, 0, 0, 54, 0, 0,
    0, 0, 0, 0, 0, 67, 0, 0, 0, 39, 0, 0, 0, 19, 0, 0, 0, 51, 0, 0, 0, 55, 0, 0, 0, 230, 0, 0, 0,
    68, 0, 0, 0, 45, 0, 0, 0, 47, 0, 0, 0, 52, 0, 0, 0, 56, 0, 10, 1, 0, 0, 0, 0, 69, 0, 0, 0, 46,
    0, 0, 0, 48, 0, 0, 0, 0, 0, 0, 0, 229, 0, 0, 0, 80, 0, 0, 0, 76, 0, 0, 0, 42, 0, 0, 0, 49, 0,
    0, 0, 40, 0, 0, 0, 82, 0, 0, 0, 81, 0, 0, 0, 74, 0, 0, 0, 77, 0, 0, 0, 75, 0, 0, 0, 78, 0, 0,
    0, 0, 0, 0, 0, 79, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// Fn-layer default matrix for id 3804, Windows (`defaultFnMatrix` in
/// `74d1dc6b.js`). Same 128-slot layout as the base matrix.
pub const FN_MATRIX_3804_WIN: [u8; 512] = [
    10, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 112, 0, 0, 0, 0,
    0, 14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 111, 0, 0, 0, 0, 0, 14, 0, 1, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 10, 3, 0, 0, 0, 0, 227, 43, 0, 0, 0, 0, 14, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 3, 0, 182, 0, 0, 0, 0, 0, 14, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0,
    205, 0, 0, 0, 0, 0, 14, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 181, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 227, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 138, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 5, 3, 0, 0, 0,
    0, 0, 3, 0, 35, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 226, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 234, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 233, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 13, 5, 0, 2, 10, 12, 0, 0, 10, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 13, 2, 1, 0, 13, 2, 2, 0,
    13, 1, 0, 0, 13, 5, 1, 0, 13, 3, 2, 0, 13, 3, 1, 0, 0, 0, 0, 0, 13, 5, 0, 1, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// Fn-layer default matrix for id 3804, macOS (`defaultFnMacMatrix` in
/// `74d1dc6b.js`). Differs from Windows at slots 17/18/42/48/54.
pub const FN_MATRIX_3804_MAC: [u8; 512] = [
    10, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 112, 0, 0, 0, 0,
    0, 14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 111, 0, 0, 0, 0, 0, 14, 0, 1, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 224, 82, 0, 0, 0, 0, 0, 14, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 3, 0, 182, 0, 0, 0, 0, 0, 14, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0,
    205, 0, 0, 0, 0, 0, 14, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 181, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 160, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 13, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 5, 3, 0, 0, 0,
    0, 0, 13, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 226, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 234, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 233, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 13, 5, 0, 2, 10, 12, 0, 0, 10, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 13, 2, 1, 0, 13, 2, 2, 0,
    13, 1, 0, 0, 13, 5, 1, 0, 13, 3, 2, 0, 13, 3, 1, 0, 0, 0, 0, 0, 13, 5, 0, 1, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// Fn-layer default matrix for id 2518 (`defaultFnMatrix` in
/// `7de29f04.js`). Byte-identical to [`FN_MATRIX_3804_WIN`].
pub const FN_MATRIX_2518_WIN: [u8; 512] = FN_MATRIX_3804_WIN;

/// The 2518 module defines no `defaultFnMacMatrix`; the shared base class
/// (`7a5b12c9.js`) assigns `defaultFnMacMatrix = this.defaultFnMatrix`.
pub const FN_MATRIX_2518_MAC: [u8; 512] = FN_MATRIX_2518_WIN;

/// Default base-layer matrix for a detected device id.
pub fn default_matrix(device_id: Option<u32>) -> &'static [u8; 512] {
    match device_id {
        Some(3804) => &MATRIX_3804,
        _ => &MATRIX_2518,
    }
}

/// Default Fn-layer matrix for a detected device id and OS.
pub fn default_fn_matrix(device_id: Option<u32>, mac: bool) -> &'static [u8; 512] {
    match (device_id, mac) {
        (Some(3804), false) => &FN_MATRIX_3804_WIN,
        (Some(3804), true) => &FN_MATRIX_3804_MAC,
        (_, false) => &FN_MATRIX_2518_WIN,
        (_, true) => &FN_MATRIX_2518_MAC,
    }
}

/// Decode one 4-byte matrix slot to a USB HID usage like JS `matrixToHid`:
/// `[0,0,hid,0] -> hid`, otherwise the packed u32 (extended keys).
pub fn matrix_to_hid(slot: [u8; 4]) -> u32 {
    if slot[0] == 0 && slot[1] == 0 && slot[3] == 0 {
        slot[2] as u32
    } else {
        ((slot[0] as u32) << 24)
            | ((slot[1] as u32) << 16)
            | ((slot[2] as u32) << 8)
            | slot[3] as u32
    }
}

/// All populated (slot, hid) pairs for a matrix, skipping `[0,0,0,0]`.
pub fn populated_keys(matrix: &[u8; 512]) -> Vec<(usize, u32)> {
    let mut out = Vec::new();
    for (slot, chunk) in matrix.chunks_exact(4).enumerate() {
        let hid = matrix_to_hid([chunk[0], chunk[1], chunk[2], chunk[3]]);
        if hid != 0 {
            out.push((slot, hid));
        }
    }
    out
}

/// Nerd-Font arrow glyphs for the arrow keys (Font Awesome codepoints
/// U+F060–U+F063, shipped in every Nerd Font). The UI loads a system Nerd
/// Font into egui so these render as real arrow icons.
pub fn arrow_glyph(hid: u32) -> Option<&'static str> {
    match hid {
        80 => Some("\u{f060}"), // arrow left
        82 => Some("\u{f062}"), // arrow up
        81 => Some("\u{f063}"), // arrow down
        79 => Some("\u{f061}"), // arrow right
        _ => None,
    }
}

/// USB HID keyboard usage names (0x04..0xE7 subset + modifiers).
pub fn hid_name(hid: u32) -> String {
    match hid {
        4 => "A".into(),
        5 => "B".into(),
        6 => "C".into(),
        7 => "D".into(),
        8 => "E".into(),
        9 => "F".into(),
        10 => "G".into(),
        11 => "H".into(),
        12 => "I".into(),
        13 => "J".into(),
        14 => "K".into(),
        15 => "L".into(),
        16 => "M".into(),
        17 => "N".into(),
        18 => "O".into(),
        19 => "P".into(),
        20 => "Q".into(),
        21 => "R".into(),
        22 => "S".into(),
        23 => "T".into(),
        24 => "U".into(),
        25 => "V".into(),
        26 => "W".into(),
        27 => "X".into(),
        28 => "Y".into(),
        29 => "Z".into(),
        30 => "1".into(),
        31 => "2".into(),
        32 => "3".into(),
        33 => "4".into(),
        34 => "5".into(),
        35 => "6".into(),
        36 => "7".into(),
        37 => "8".into(),
        38 => "9".into(),
        39 => "0".into(),
        40 => "Enter".into(),
        41 => "Esc".into(),
        42 => "Backspace".into(),
        43 => "Tab".into(),
        44 => "Space".into(),
        45 => "-".into(),
        46 => "=".into(),
        47 => "[".into(),
        48 => "]".into(),
        49 => "\\".into(),
        51 => ";".into(),
        52 => "'".into(),
        53 => "`".into(),
        54 => ",".into(),
        55 => ".".into(),
        56 => "/".into(),
        57 => "CapsLock".into(),
        58 => "F1".into(),
        59 => "F2".into(),
        60 => "F3".into(),
        61 => "F4".into(),
        62 => "F5".into(),
        63 => "F6".into(),
        64 => "F7".into(),
        65 => "F8".into(),
        66 => "F9".into(),
        67 => "F10".into(),
        68 => "F11".into(),
        69 => "F12".into(),
        73 => "Ins".into(),
        74 => "Home".into(),
        75 => "PgUp".into(),
        76 => "Del".into(),
        77 => "End".into(),
        78 => "PgDn".into(),
        79 => "Right".into(),
        80 => "Left".into(),
        81 => "Down".into(),
        82 => "Up".into(),
        70 => "PrtSc".into(),
        71 => "ScrLk".into(),
        72 => "Pause".into(),
        83 => "NumLk".into(),
        84 => "Kp/".into(),
        85 => "Kp*".into(),
        86 => "Kp-".into(),
        87 => "Kp+".into(),
        88 => "KpEnt".into(),
        89 => "Kp1".into(),
        90 => "Kp2".into(),
        91 => "Kp3".into(),
        92 => "Kp4".into(),
        93 => "Kp5".into(),
        94 => "Kp6".into(),
        95 => "Kp7".into(),
        96 => "Kp8".into(),
        97 => "Kp9".into(),
        98 => "Kp0".into(),
        99 => "Kp.".into(),
        100 => "NonUS".into(),
        101 => "App".into(),
        102 => "Power".into(),
        103 => "Kp=".into(),
        104 => "F13".into(),
        105 => "F14".into(),
        106 => "F15".into(),
        107 => "F16".into(),
        108 => "F17".into(),
        109 => "F18".into(),
        110 => "F19".into(),
        111 => "F20".into(),
        112 => "F21".into(),
        113 => "F22".into(),
        114 => "F23".into(),
        115 => "F24".into(),
        116 => "Exec".into(),
        117 => "Help".into(),
        118 => "Menu".into(),
        127 => "Mute".into(),
        128 => "Vol+".into(),
        129 => "Vol-".into(),
        224 => "LCtrl".into(),
        225 => "LShift".into(),
        226 => "LAlt".into(),
        227 => "LGui".into(),
        228 => "RCtrl".into(),
        229 => "RShift".into(),
        230 => "RAlt".into(),
        231 => "RGui".into(),
        167_837_696 => "Fn".into(), // [10,1,0,0] layer key
        _ if hid > 255 => format!("EXT({hid:#X})"),
        _ => format!("HID({hid})"),
    }
}

#[cfg(test)]
mod tests {
    use super::arrow_glyph;

    #[test]
    fn arrows_have_nerd_glyphs() {
        assert_eq!(arrow_glyph(80), Some("\u{f060}"));
        assert_eq!(arrow_glyph(82), Some("\u{f062}"));
        assert_eq!(arrow_glyph(81), Some("\u{f063}"));
        assert_eq!(arrow_glyph(79), Some("\u{f061}"));
        assert_eq!(arrow_glyph(4), None);
    }

    use super::*;

    #[test]
    fn sk75_matrices_have_81ish_keys() {
        for m in [&MATRIX_2518, &MATRIX_3804] {
            let keys = populated_keys(m);
            // 81-key layout + a few extended markers; both have 81 printable + markers.
            assert!(keys.len() >= 80 && keys.len() <= 90, "got {}", keys.len());
            assert!(keys.iter().any(|(_, h)| *h == 41)); // Esc
            assert!(keys.iter().any(|(_, h)| *h == 44)); // Space
        }
    }

    #[test]
    fn variants_differ_only_at_marker_slots() {
        let mut diffs = 0;
        for (a, b) in MATRIX_2518.iter().zip(MATRIX_3804.iter()) {
            if a != b {
                diffs += 1;
            }
        }
        assert!(diffs > 0 && diffs <= 8, "diffs={diffs}");
    }

    fn quad(m: &[u8; 512], slot: usize) -> [u8; 4] {
        [
            m[slot * 4],
            m[slot * 4 + 1],
            m[slot * 4 + 2],
            m[slot * 4 + 3],
        ]
    }

    #[test]
    fn fn_defaults_match_driver_tables() {
        // 74d1dc6b.js defaultFnMatrix / defaultFnMacMatrix.
        assert_eq!(quad(&FN_MATRIX_3804_WIN, 0), [10, 2, 0, 0]); // Fn reset
        assert_eq!(quad(&FN_MATRIX_3804_WIN, 18), [0, 0, 227, 43]); // LGui+Tab
        assert_eq!(quad(&FN_MATRIX_3804_WIN, 42), [0, 0, 227, 8]); // LGui+E
        assert_eq!(quad(&FN_MATRIX_3804_MAC, 18), [0, 224, 82, 0]); // LCtrl+Up
        assert_eq!(quad(&FN_MATRIX_3804_MAC, 42), [3, 0, 160, 2]); // mac search
        assert_eq!(quad(&FN_MATRIX_3804_MAC, 17), [0, 0, 0, 0]);
        // 2518 has only a win table, byte-identical to 3804's; the shared
        // base class falls back to it for mac.
        assert_eq!(FN_MATRIX_2518_WIN, FN_MATRIX_3804_WIN);
        assert_eq!(FN_MATRIX_2518_MAC, FN_MATRIX_2518_WIN);
    }

    #[test]
    fn default_matrix_picks_by_device_id() {
        assert_eq!(default_matrix(Some(2518)), &MATRIX_2518);
        assert_eq!(default_matrix(Some(3804)), &MATRIX_3804);
        assert_eq!(default_matrix(None), &MATRIX_2518);
        assert_eq!(default_fn_matrix(Some(2518), false), &FN_MATRIX_2518_WIN);
        assert_eq!(default_fn_matrix(Some(2518), true), &FN_MATRIX_2518_MAC);
        assert_eq!(default_fn_matrix(Some(3804), false), &FN_MATRIX_3804_WIN);
        assert_eq!(default_fn_matrix(Some(3804), true), &FN_MATRIX_3804_MAC);
    }
}
