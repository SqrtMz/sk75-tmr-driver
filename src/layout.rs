//! On-screen keyboard geometry for SK75 TMR.
//!
//! Positions come straight from the original driver's keymap asset
//! `dist/js/7d1d89fb.js` (`keyboard_81_sk75tmr_keymappings_ui_info`):
//! a 732x281 canvas with per-key `{x,y,width,height,displayText}` rects.
//! Each entry also carries the USB HID usage so a rect resolves to the
//! same matrix slot the firmware's analog/config reports use.
//! REA: `analyze_javascript_application` summary `ev_927021fe...`
//! + module page `ev_6b3387ab...`.

/// Canvas size in the original asset.
pub const CANVAS_W: f32 = 732.0;
pub const CANVAS_H: f32 = 281.0;

/// (layout id, HID usage, x, y, w, h, label).
pub const KEYS: [(&str, u32, f32, f32, f32, f32, &str); 81] = [
    ("ArrowRight", 79, 691.0, 240.0, 40.0, 40.0, "→"),
    ("ArrowDown", 81, 645.0, 240.0, 40.0, 40.0, "↓"),
    ("ArrowLeft", 80, 599.0, 240.0, 40.0, 40.0, "←"),
    ("AltRight", 230, 461.0, 240.0, 48.0, 40.0, "Alt"),
    ("Fn", 167_837_696, 515.0, 240.0, 48.0, 40.0, "Fn"),
    ("Space", 44, 163.0, 240.0, 292.0, 40.0, "Space"),
    ("AltLeft", 226, 109.0, 240.0, 48.0, 40.0, "Alt"),
    ("MetaLeft", 227, 55.0, 240.0, 48.0, 40.0, "Win"),
    ("ControlLeft", 224, 1.0, 240.0, 48.0, 40.0, "Ctrl"),
    ("ArrowUp", 82, 645.0, 194.0, 40.0, 40.0, "↑"),
    ("ShiftRight", 229, 560.0, 194.0, 79.0, 40.0, "Shift"),
    ("Slash", 56, 514.0, 194.0, 40.0, 40.0, "/?"),
    ("Period", 55, 468.0, 194.0, 40.0, 40.0, ".>"),
    ("Comma", 54, 422.0, 194.0, 40.0, 40.0, ",<"),
    ("KeyM", 16, 376.0, 194.0, 40.0, 40.0, "M"),
    ("KeyN", 17, 330.0, 194.0, 40.0, 40.0, "N"),
    ("KeyB", 5, 284.0, 194.0, 40.0, 40.0, "B"),
    ("KeyV", 25, 238.0, 194.0, 40.0, 40.0, "V"),
    ("KeyC", 6, 192.0, 194.0, 40.0, 40.0, "C"),
    ("KeyX", 27, 146.0, 194.0, 40.0, 40.0, "X"),
    ("KeyZ", 29, 100.0, 194.0, 40.0, 40.0, "Z"),
    ("ShiftLeft", 225, 1.0, 194.0, 93.0, 40.0, "Shift"),
    ("PageDown", 78, 691.0, 148.0, 40.0, 40.0, "PgDn"),
    ("Enter", 40, 585.0, 148.0, 100.0, 40.0, "Enter"),
    ("Quote", 52, 539.0, 148.0, 40.0, 40.0, "‘”"),
    ("Semicolon", 51, 493.0, 148.0, 40.0, 40.0, ";:"),
    ("KeyL", 15, 447.0, 148.0, 40.0, 40.0, "L"),
    ("KeyK", 14, 401.0, 148.0, 40.0, 40.0, "K"),
    ("KeyJ", 13, 355.0, 148.0, 40.0, 40.0, "J"),
    ("KeyH", 11, 309.0, 148.0, 40.0, 40.0, "H"),
    ("KeyG", 10, 263.0, 148.0, 40.0, 40.0, "G"),
    ("KeyF", 9, 217.0, 148.0, 40.0, 40.0, "F"),
    ("KeyD", 7, 171.0, 148.0, 40.0, 40.0, "D"),
    ("KeyS", 22, 125.0, 148.0, 40.0, 40.0, "S"),
    ("KeyA", 4, 79.0, 148.0, 40.0, 40.0, "A"),
    ("CapsLock", 57, 1.0, 148.0, 72.0, 40.0, "Caps"),
    ("PageUp", 75, 691.0, 102.0, 40.0, 40.0, "PgUp"),
    ("Backslash", 49, 625.0, 102.0, 61.0, 40.0, "\\|"),
    ("BracketRight", 48, 579.0, 102.0, 40.0, 40.0, "]}"),
    ("BracketLeft", 47, 533.0, 102.0, 40.0, 40.0, "[{"),
    ("KeyP", 19, 487.0, 102.0, 40.0, 40.0, "P"),
    ("KeyO", 18, 441.0, 102.0, 40.0, 40.0, "O"),
    ("KeyI", 12, 395.0, 102.0, 40.0, 40.0, "I"),
    ("KeyU", 24, 349.0, 102.0, 40.0, 40.0, "U"),
    ("KeyY", 28, 303.0, 102.0, 40.0, 40.0, "Y"),
    ("KeyT", 23, 257.0, 102.0, 40.0, 40.0, "T"),
    ("KeyR", 21, 211.0, 102.0, 40.0, 40.0, "R"),
    ("KeyE", 8, 165.0, 102.0, 40.0, 40.0, "E"),
    ("KeyW", 26, 119.0, 102.0, 40.0, 40.0, "W"),
    ("KeyQ", 20, 73.0, 102.0, 40.0, 40.0, "Q"),
    ("Tab", 43, 1.0, 102.0, 66.0, 40.0, "Tab"),
    ("End", 77, 691.0, 56.0, 40.0, 40.0, "End"),
    ("Backspace", 42, 599.0, 56.0, 86.0, 40.0, "BackSpace"),
    ("Equal", 46, 553.0, 56.0, 40.0, 40.0, "=+"),
    ("Minus", 45, 507.0, 56.0, 40.0, 40.0, "-_"),
    ("Digit0", 39, 461.0, 56.0, 40.0, 40.0, "0)"),
    ("Digit9", 38, 415.0, 56.0, 40.0, 40.0, "9("),
    ("Digit8", 37, 369.0, 56.0, 40.0, 40.0, "8*"),
    ("Digit7", 36, 323.0, 56.0, 40.0, 40.0, "7&"),
    ("Digit6", 35, 277.0, 56.0, 40.0, 40.0, "6^"),
    ("Digit5", 34, 231.0, 56.0, 40.0, 40.0, "5%"),
    ("Digit4", 33, 185.0, 56.0, 40.0, 40.0, "4$"),
    ("Digit3", 32, 139.0, 56.0, 40.0, 40.0, "3#"),
    ("Digit2", 31, 93.0, 56.0, 40.0, 40.0, "2@"),
    ("Digit1", 30, 47.0, 56.0, 40.0, 40.0, "1!"),
    ("Backquote", 53, 1.0, 56.0, 40.0, 40.0, "`~"),
    ("Home", 74, 691.0, 1.0, 40.0, 40.0, "Home"),
    ("Delete", 76, 645.0, 1.0, 40.0, 40.0, "Del"),
    ("F12", 69, 586.0, 1.0, 40.0, 40.0, "F12"),
    ("F11", 68, 540.0, 1.0, 40.0, 40.0, "F11"),
    ("F10", 67, 494.0, 1.0, 40.0, 40.0, "F10"),
    ("F9", 66, 448.0, 1.0, 40.0, 40.0, "F9"),
    ("F8", 65, 391.0, 1.0, 40.0, 40.0, "F8"),
    ("F7", 64, 345.0, 1.0, 40.0, 40.0, "F7"),
    ("F6", 63, 299.0, 1.0, 40.0, 40.0, "F6"),
    ("F5", 62, 253.0, 1.0, 40.0, 40.0, "F5"),
    ("F4", 61, 196.0, 1.0, 40.0, 40.0, "F4"),
    ("F3", 60, 150.0, 1.0, 40.0, 40.0, "F3"),
    ("F2", 59, 104.0, 1.0, 40.0, 40.0, "F2"),
    ("F1", 58, 58.0, 1.0, 40.0, 40.0, "F1"),
    ("Escape", 41, 1.0, 1.0, 40.0, 40.0, "Esc"),
];

/// Resolve a layout key to its firmware matrix slot.
///
/// The Fn key is not a HID usage: both SK75 matrices encode it as the
/// extended marker `[10,1,0,0]`, which packs to `167837696` (`0x0A010000`)
/// exactly like the original `matrixToHid`/keymap asset.
pub fn slot_for(hid: u32, matrix: &[u8; 512]) -> Option<usize> {
    if hid == 0 {
        return None;
    }
    matrix
        .chunks_exact(4)
        .position(|c| crate::matrix::matrix_to_hid([c[0], c[1], c[2], c[3]]) == hid)
}

/// Every layout key's matrix slot, in board order. Both SK75 variants expose
/// exactly 81 populated slots (80 HID keys + the Fn marker).
pub fn all_slots(matrix: &[u8; 512]) -> Vec<usize> {
    KEYS.iter().filter_map(|k| slot_for(k.1, matrix)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_81_keys_present() {
        assert_eq!(KEYS.len(), 81);
        assert_eq!((CANVAS_W, CANVAS_H), (732.0, 281.0));
    }

    #[test]
    fn space_is_wide_and_mapped() {
        let space = KEYS.iter().find(|k| k.0 == "Space").unwrap();
        assert!(space.4 > 200.0);
        let slot = slot_for(44, &crate::matrix::MATRIX_2518).unwrap();
        assert!(crate::matrix::populated_keys(&crate::matrix::MATRIX_2518)
            .iter()
            .any(|(s, _)| *s == slot));
    }

    #[test]
    fn fn_maps_to_extended_marker_slot() {
        assert_eq!(slot_for(167_837_696, &crate::matrix::MATRIX_2518), Some(65));
        assert_eq!(slot_for(167_837_696, &crate::matrix::MATRIX_3804), Some(71));
        assert_eq!(slot_for(0, &crate::matrix::MATRIX_2518), None);
    }

    #[test]
    fn every_layout_key_has_a_unique_slot() {
        // 3804 is the host variant and maps all 81 rects (80 HID keys + Fn).
        // 2518's matrix has RCtrl (228) where the shared UI geometry has
        // AltRight (230), so that one rect has no slot on 2518.
        for (matrix, expected) in [
            (&crate::matrix::MATRIX_2518, 80),
            (&crate::matrix::MATRIX_3804, 81),
        ] {
            let slots = all_slots(matrix);
            assert_eq!(slots.len(), expected);
            let mut unique = slots.clone();
            unique.sort_unstable();
            unique.dedup();
            assert_eq!(unique.len(), expected);
            assert!(slots.contains(&layout_fn_slot(matrix)));
        }
    }

    fn layout_fn_slot(matrix: &[u8; 512]) -> usize {
        crate::matrix::populated_keys(matrix)
            .into_iter()
            .find(|(_, h)| *h == 167_837_696)
            .map(|(s, _)| s)
            .unwrap()
    }
}
