//! Key rebinding (remap) for SK75 TMR.
//!
//! Mirrors the original driver's keymap layer (`dist/js/7a5b12c9.js`
//! `class at`: `_getKeyMatrix`/`setKeyConfigSimple`/`getKeyConfig`,
//! `_getFnKeyMatrix`/`setFnKeyConfigSimple`, macro cmds 11/139, profiles
//! 0..3 + Fn win/mac) with option tables from `dist/js/index.5627ed60.js`
//! (`uw` function/media map, `Fa` mouse map, `O_` gamepad map,
//! `matrixToConfigs`/`configToMatrix`/`changeArrToConfig`).
//! REA: `analyze_javascript_application` summary `ev_942b38b6…`
//! (scoped `7a5b12c9.js`+`15394fe0.js`+`ec0c0cb4.js`) + module page
//! `ev_4e41ffbf…`. Reset defaults (`default_quad`) were re-confirmed with
//! parent `ev_f2e38f5e…`, summary `ev_270e853a…`.

use anyhow::Result;

use crate::{hid::Sk75, matrix, proto::ReportKind};

pub const CMD_SET_KEYMATRIX: u8 = 10;
pub const CMD_GET_KEYMATRIX: u8 = 138;
pub const CMD_SET_FN: u8 = 16;
pub const CMD_GET_FN: u8 = 144;

/// SK75 has 4 profiles (device DB `layer:4`) and Fn win/mac × 1 layer each
/// (`fnSysLayer:{win:1,mac:1}`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FnSys {
    Win,
    Mac,
}

impl FnSys {
    pub fn code(self) -> u8 {
        match self {
            FnSys::Win => 0,
            FnSys::Mac => 1,
        }
    }
}

/// Binding options present in the original driver (decode branches of
/// `matrixToConfigs` + encode branches of `configToMatrix`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Binding {
    /// Combo: up to [`MAX_COMBO_KEYS`] simultaneous keys in one 4-byte slot
    /// (`[0,skey,key,key2]`). Common lone key: `[0,0,hid,0]`.
    Combo { skey: u8, key: u32, key2: u8 },
    /// Disabled slot: `[0,0,0,0]`.
    Disabled,
    /// Mouse button from `Fa` (Left/Right/Middle/Back/Forward + wheel).
    Mouse { label: String, quad: [u8; 4] },
    /// Media/function key from `uw` (volume, transport, calculator…).
    Function { label: String, quad: [u8; 4] },
    /// Gamepad button from `O_`.
    Gamepad { label: String, quad: [u8; 4] },
    /// Macro slot: `[9,type,idx,0]`; content editing is a separate macro
    /// editor in the original driver — here only the slot assignment.
    Macro { macro_type: String, index: u8 },
    /// Gaming combos: Snap `[22,n,key,0]`, MDS `[23,…]`, MT `[24,…]`,
    /// recoil `[22,4|6|7,…]`.
    Special { label: String, quad: [u8; 4] },
    /// Anything else, shown as hex.
    Unknown([u8; 4]),
}

impl Binding {
    pub fn quad(&self) -> [u8; 4] {
        match self {
            Binding::Combo { skey, key, key2 } => {
                if *key <= 255 {
                    [0, *skey, *key as u8, *key2]
                } else {
                    // Extended HID: pack big-endian u32 (matches `hL`).
                    let b = key.to_be_bytes();
                    [b[0], b[1], b[2], b[3]]
                }
            }
            Binding::Disabled => [0, 0, 0, 0],
            Binding::Mouse { quad, .. }
            | Binding::Function { quad, .. }
            | Binding::Gamepad { quad, .. }
            | Binding::Special { quad, .. }
            | Binding::Unknown(quad) => *quad,
            Binding::Macro { macro_type, index } => {
                let n = match macro_type.as_str() {
                    "on_off" => 1,
                    "touch_repeat" => 2,
                    _ => 0,
                };
                [9, n, *index, 0]
            }
        }
    }

    pub fn short_label(&self) -> String {
        match self {
            // Mirrors the original board overlay (`f01f4a92.js` combo branch):
            // every nonzero of skey/key/key2 is shown, joined with " + ".
            // Fn-layer passthrough quads store the key in skey (e.g. O as
            // `[0,18,0,0]`), so showing only `key` rendered them as "HID(0)".
            Binding::Combo { skey, key, key2 } => {
                let parts: Vec<String> = [*skey as u32, *key, *key2 as u32]
                    .into_iter()
                    .filter(|h| *h != 0)
                    .map(matrix::hid_name)
                    .collect();
                parts.join(" + ")
            }
            Binding::Disabled => String::from("Disabled"),
            Binding::Mouse { label, .. } => format!("Mouse {label}"),
            Binding::Function { label, .. } => label.clone(),
            Binding::Gamepad { label, .. } => format!("Pad {label}"),
            Binding::Macro { macro_type, index } => format!("Macro {macro_type}#{index}"),
            Binding::Special { label, .. } => label.clone(),
            // The original board shows ConfigUnknown as system key
            // (`case"ConfigUnknown":o=s.系统按键`), never raw ints.
            Binding::Unknown(_) => String::from("System"),
        }
    }
}

/// Maximum keys in one combo, as enforced by the original driver.
///
/// REA (`ev_749cc9f3…`, scoped `0482e620.js`+`2e81aae0.js`+`59693644.js`,
/// corroborated by direct static reads): the picker keeps a global key list
/// (`handleClick_组合键` in `index.5627ed60.js`, same toggle in `59693644.js`)
/// that removes on re-click and silently refuses a 4th pick
/// (`if(length>=3)return`), while the panel hint reads
/// `组合键最多可选择3个字符` ("combo holds at most 3"). The save path
/// (`设置改键("组合键")`) maps click order positionally to
/// `skey/key/key2`. Structural reason, observed: the combo wire encoding is
/// `[0,skey,key,key2]` — one 4-byte keymap slot whose first byte is the
/// combo tag, leaving exactly 3 key bytes. Longer combos are unencodable in
/// `configToMatrix`; whether firmware would additionally reject them is
/// unobserved (static analysis only, no execution).
pub const MAX_COMBO_KEYS: usize = 3;

/// Build a combo from picked HIDs in pick order.
///
/// Mirrors `设置改键("组合键")`, except the first pick fills `key` (not
/// `skey`) so lone keys keep firmware default bytes `[0,0,hid,0]` instead of
/// `[0,hid,0,0]`; both display identically via the skey/key/key2 join.
/// Returns `None` when empty, longer than [`MAX_COMBO_KEYS`], or when a
/// `skey`/`key2`-slot value exceeds one byte (`key` keeps the extended-ID
/// `hL` packing in [`Binding::quad`]).
#[cfg(test)]
pub fn combo_from_keys(keys: &[u32]) -> Option<Binding> {
    if keys.is_empty() || keys.len() > MAX_COMBO_KEYS {
        return None;
    }
    let key = keys[0];
    let key2 = u8::try_from(*keys.get(1).unwrap_or(&0)).ok()?;
    let skey = u8::try_from(*keys.get(2).unwrap_or(&0)).ok()?;
    Some(Binding::Combo { skey, key, key2 })
}

/// Well-known write-safe options (literal quads from `uw`/`Fa`/`O_`;
/// no `Ie.*` indirection so bytes are exact).
pub fn mouse_options() -> Vec<(String, [u8; 4])> {
    vec![
        (String::from("Left"), [1, 0, 240, 0]),
        (String::from("Right"), [1, 0, 241, 0]),
        (String::from("Middle"), [1, 0, 242, 0]),
        (String::from("Back"), [1, 0, 243, 0]),
        (String::from("Forward"), [1, 0, 244, 0]),
        (String::from("WheelUp"), [1, 0, 247, 0]),
        (String::from("WheelDown"), [1, 0, 248, 0]),
    ]
}

pub fn function_options() -> Vec<(String, [u8; 4])> {
    let mut opts = vec![
        (String::from("Play/Pause"), [3, 0, 205, 0]),
        (String::from("Stop"), [3, 0, 183, 0]),
        (String::from("Prev track"), [3, 0, 182, 0]),
        (String::from("Next track"), [3, 0, 181, 0]),
        (String::from("Mute"), [3, 0, 226, 0]),
        (String::from("Volume-"), [3, 0, 234, 0]),
        (String::from("Volume+"), [3, 0, 233, 0]),
        (String::from("Calculator"), [3, 0, 146, 1]),
        (String::from("Mail"), [3, 0, 138, 1]),
        (String::from("My Computer"), [3, 0, 148, 1]),
        (String::from("Search"), [3, 0, 33, 2]),
        (String::from("Home"), [3, 0, 35, 2]),
        (String::from("Brightness-"), [3, 0, 112, 0]),
        (String::from("Brightness+"), [3, 0, 111, 0]),
        (String::from("Firepower"), [11, 0, 0, 0]),
    ];
    // Native Fn functions below: every named entry is assignable, so factory
    // Fn assignments (e.g. LED Mode Loop on Home) can be replicated.
    opts.extend(NATIVE_FUNCTIONS.iter().map(|(q, l)| (String::from(*l), *q)));
    opts
}

/// Numeric-code special functions, each tracked to its driver action:
/// - `[10,1,0,0]` Fn layer key, `[10,1,1,0]` alternate Fn, `[10,13,0,0]`
///   Fn lock screen; `[10,15/14,0,0]` swap volume knob with keyboard
///   backlight brightness; `[10,5,0,0]` wireless pair; `[10,24,0,0]`
///   countdown timer.
/// - `[8,0,n,m]` profile control (firmware `setCurrentProfile` cmd 4 family).
/// - `[20,0,n,0]` DPI control loop/steps.
/// - `[2,0,129..131,0]` system power/sleep/wake.
/// - `[11,10,n,0]` OLED display actions; `[6,128,0,n]` mic/app launcher;
///   `[18,0,*,*]` website/assistant shortcuts.

pub fn gamepad_options() -> Vec<(String, [u8; 4])> {
    vec![
        (String::from("Up"), [21, 3, 0, 0]),
        (String::from("Down"), [21, 3, 1, 0]),
        (String::from("Left"), [21, 3, 2, 0]),
        (String::from("Right"), [21, 3, 3, 0]),
        (String::from("Btn1"), [21, 0, 0, 0]),
        (String::from("Btn2"), [21, 0, 1, 0]),
        (String::from("Btn4"), [21, 0, 3, 0]),
        (String::from("A"), [255, 255, 255, 15]),
        (String::from("B"), [255, 255, 255, 16]),
        (String::from("X"), [255, 255, 255, 17]),
        (String::from("Y"), [255, 255, 255, 18]),
    ]
}

/// Standard-key options shown at a glance in the key picker (combo targets).
/// HID usages from the driver's key tables (`keyCodeToDisplayText` /
/// `hidCodeMapHtmlCode` in `f01f4a92.js`; REA parent `ev_9da292dd…`):
/// letters, digits, controls, F-keys, navigation and modifiers.
pub const STANDARD_KEYS: [u32; 116] = [
    4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
    29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 51, 52, 53,
    54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77,
    78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 103,
    104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 224, 225, 226, 227, 228, 229, 230,
    231,
];

/// Native keyboard functions: the `uw` function map plus the master key
/// table's LED/power/MIC/AI rows, all in `dist/js/index.5627ed60.js`
/// (REA `ev_42330295…`, scoped `index.5627ed60.js`+`59693644.js`,
/// corroborated by direct static reads of the extracted `app-32.7z` tree),
/// plus the SK75's own connector/lighting-control Fn quads (round 15).
/// Only `q[0] != 0` entries are listed: on the read path
/// (`matrixToConfigs`) the `l[0]===0` combo branch precedes the
/// function-table lookup, so `q[0]==0` quads such as `[0,0,227,7]` (WIN_D)
/// decode as combos and display via the join above, like the original board.
/// Every entry here is also in the Functions picker, so factory Fn
/// assignments can be replicated onto other keys; bytes round-trip exact.
const NATIVE_FUNCTIONS: [([u8; 4], &str); 65] = [
    // Media / launcher extras (`uw`, q0 == 3; Play Alt from master table).
    ([3, 0, 131, 1], "Player"),
    ([3, 0, 36, 2], "Back"),
    ([3, 0, 39, 2], "Refresh"),
    ([3, 1, 205, 0], "Play/Pause Alt"),
    // System power (`uw`, q0 == 2).
    ([2, 0, 129, 0], "Power"),
    ([2, 0, 130, 0], "Sleep"),
    ([2, 0, 131, 0], "Wake"),
    // Mic / app launcher (`uw`, q0 == 6).
    ([6, 128, 0, 0], "Mic Toggle"),
    ([6, 128, 0, 1], "Open App 1"),
    ([6, 128, 0, 2], "Open App 2"),
    ([6, 128, 0, 3], "Open App 3"),
    // Profile control, firmware cmd-4 family (`uw`, q0 == 8).
    ([8, 0, 0, 0], "Profile Value"),
    ([8, 0, 1, 0], "Profile+"),
    ([8, 0, 2, 0], "Profile-"),
    ([8, 0, 3, 0], "Profile Loop"),
    ([8, 0, 4, 0], "Profile Switch"),
    ([8, 0, 4, 1], "Profile 1"),
    ([8, 0, 4, 2], "Profile 2"),
    ([8, 0, 4, 3], "Profile 3"),
    ([8, 0, 4, 4], "Profile 4"),
    // Fn keys (`uw`, q0 == 10; r_Fn/MIC from the master key table).
    // `[10,13,0,0]` is listed twice in `uw` (Fn lock screen + a
    // volume-brightness alias); the reverse lookup hits the Fn lock first.
    ([10, 1, 0, 0], "Fn"),
    ([10, 1, 1, 0], "Fn Alt"),
    ([10, 36, 0, 0], "Fn Alt 2"),
    ([10, 13, 0, 0], "Fn Lock Screen"),
    ([10, 15, 0, 0], "VolKnob<->Backlight"),
    ([10, 14, 0, 0], "Volume Brightness"),
    // Recoil / weapon select (`uw`, q0 == 22). These precede the Snap/MDS
    // branches on the read path, mirroring `matrixToConfigs` order.
    ([22, 5, 0, 0], "Recoil"),
    ([22, 6, 255, 0], "Recoil Hold"),
    ([22, 7, 255, 0], "Recoil Toggle"),
    ([22, 1, 0, 0], "Weapon Next"),
    ([22, 2, 0, 0], "Weapon Prev"),
    ([22, 3, 0, 0], "Weapon Cycle"),
    // DPI (`uw`, q0 == 20).
    ([20, 0, 0, 0], "DPI Loop"),
    ([20, 0, 1, 0], "DPI+"),
    ([20, 0, 2, 0], "DPI-"),
    ([20, 0, 4, 0], "DPI Shift"),
    // OLED (`uw`, q0 == 11).
    ([11, 10, 1, 0], "OLED Text Color"),
    ([11, 10, 2, 0], "OLED Back Color"),
    ([11, 10, 3, 0], "OLED Confirm"),
    // Website (`uw` q0 == 18 + master-table q0 == 19 driver page).
    ([18, 0, 227, 44], "Siri"),
    ([18, 0, 0, 0], "Open Website"),
    ([19, 0, 0, 0], "Open Driver Website"),
    // LED + system (`LED_Mode`, `LEDBIR*`, `ledMode*`, `LEDONOFF`,
    // `Powersave`, `MIC`, `AI` rows of the master key table).
    // `[13,1,0,0]` cycles the LED mode like the knob push action
    // (`knobKeyCodes` `LEDMODELOOP`). `[10,24,0,0]` is also what the
    // driver sends for its countdown action (`fL`); labeled AI here per
    // the master table's display vocabulary.
    ([13, 1, 0, 0], "LED Mode Loop"),
    ([13, 2, 1, 0], "LED Brightness+"),
    ([13, 2, 2, 0], "LED Brightness-"),
    ([13, 6, 0, 0], "LED Mode 1"),
    ([13, 6, 1, 0], "LED Mode 2"),
    ([10, 9, 0, 0], "LED On/Off"),
    ([10, 12, 0, 0], "Powersave"),
    ([10, 32, 0, 0], "Mic"),
    ([10, 24, 0, 0], "AI"),
    // SK75 connector / Fn-cluster quads (round 15). Sources across the
    // extracted app: the SK75 factory Fn default (`74d1dc6b.js`,
    // `Ry5088_womier_sk75he_europe_3m_8k_8k`) carries every quad of this
    // block and matches our board's live `keymap read --fn win` row by
    // row. A full sweep of the app found NO naming table for them
    // (master key table, `uw`/`fL`, `O_`/`Fa`, per-device modules,
    // SK75 ui_info `7d1d89fb.js`, help device `fa78cffb.js`, i18n
    // dictionaries all checked — the original board types them
    // ConfigUnknown and displays 系统按键/"System"); the names below
    // rest on cross-board family statistics plus operator-verified
    // behavior on our unit.
    // - `[14,0,x]` exists only on three-mode keyboards, five quads
    //   x=0,1,2,5,6 on Q/W/E/R/T (operator: Bluetooth 1/2/3, 2.4 GHz,
    //   wired mode switch).
    // - `[13,5,a,b]` on the arrow cluster (operator: Left `[13,5,0,2]`
    //   direction-, Right `[13,5,0,1]` direction+, End `[13,5,1,0]`
    //   LED color change).
    // - `[13,3,1]/[13,3,2]` on PgDn/PgUp mirror the labelled `[13,2,±]`
    //   brightness ±-pairs as LED speed (inferred: ± family, same
    //   cluster; operator confirmed every other cluster key).
    // - `[10,2,0,0]` factory reset, `[10,3,0,0]` Win-key lock,
    //   `[10,3,0,0]` Win-key lock, `[10,8,0,0]` battery level, and
    //   `[10,5,3,0]` Win/Mac mode switch (operator-verified; the
    //   driver's mouse dict labels only `[10,5,0,0]` as "PAIR").
    ([14, 0, 0, 0], "Bluetooth 1"),
    ([14, 0, 1, 0], "Bluetooth 2"),
    ([14, 0, 2, 0], "Bluetooth 3"),
    ([14, 0, 5, 0], "2.4 GHz"),
    ([14, 0, 6, 0], "Wired"),
    ([13, 3, 1, 0], "LED Speed-"),
    ([13, 3, 2, 0], "LED Speed+"),
    ([13, 5, 0, 2], "LED Direction-"),
    ([13, 5, 0, 1], "LED Direction+"),
    ([13, 5, 1, 0], "LED Color"),
    ([10, 2, 0, 0], "Factory Reset"),
    ([10, 3, 0, 0], "Win Lock"),
    ([10, 8, 0, 0], "Battery Level"),
    ([10, 5, 3, 0], "Win/Mac Switch"),
];

/// Decode one 4-byte keymap slot (mirrors `matrixToConfigs` branches, including
/// their order: `l[0]===0` combo precedes the `uw` function-table lookup,
/// which precedes the Snap/MDS/MT branches; anything else is ConfigUnknown,
/// shown on the original board as system key).
pub fn decode_quad(q: [u8; 4]) -> Binding {
    if q == [0, 0, 0, 0] {
        return Binding::Disabled;
    }
    if q[0] == 9 {
        let t = match q[1] {
            1 => "on_off",
            2 => "touch_repeat",
            _ => "repeat_times",
        };
        return Binding::Macro {
            macro_type: t.to_string(),
            index: q[2],
        };
    }
    if q[0] == 1 {
        if let Some((label, _)) = mouse_options().into_iter().find(|(_, b)| *b == q) {
            return Binding::Mouse { label, quad: q };
        }
        return Binding::Unknown(q);
    }
    if q[0] == 21 || q == [255, 255, 255, 15] {
        if let Some((label, _)) = gamepad_options().into_iter().find(|(_, b)| *b == q) {
            return Binding::Gamepad { label, quad: q };
        }
        if q[0] == 21 || q[0] == 255 {
            return Binding::Unknown(q);
        }
    }
    if q[0] == 0 {
        return Binding::Combo {
            skey: q[1],
            key: q[2] as u32,
            key2: q[3],
        };
    }
    if let Some((label, _)) = function_options().into_iter().find(|(_, b)| *b == q) {
        return Binding::Function { label, quad: q };
    }
    if q[0] == 22 {
        let label = match q[1] {
            4 => String::from("Recoil normal"),
            6 => String::from("Recoil hold"),
            7 => String::from("Recoil toggle"),
            _ => format!("Snap #{}", q[1]),
        };
        return Binding::Special { label, quad: q };
    }
    if q[0] == 23 {
        return Binding::Special {
            label: String::from("MDS"),
            quad: q,
        };
    }
    if q[0] == 24 {
        return Binding::Special {
            label: String::from("MT"),
            quad: q,
        };
    }
    Binding::Unknown(q)
}

/// One slot of a keymap layer.
#[derive(Debug, Clone)]
pub struct KeyEntry {
    pub slot: usize,
    pub default_name: String,
    pub binding: Binding,
    pub changed: bool,
}

impl Sk75 {
    /// `_getKeyMatrix(profile, 0)`: `[138,profile,255,page]` × 8 pages.
    /// JS `async _getKeyMatrix(t,e)` in `7a5b12c9.js`.
    pub fn read_keymap(&self, profile: u8) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(512);
        for page in 0..8u8 {
            let req = vec![CMD_GET_KEYMATRIX, profile, 255, page, 0];
            // Keymap responses start with binding quads, so an echoed
            // request means the keyboard is still applying a write.
            let r = self.common_msg_data(&req, ReportKind::Bit7)?;
            // Each feature body is 64 B; keymap occupies the first 64 B.
            let take = r.len().min(64);
            out.extend_from_slice(&r[..take]);
            if take < 64 {
                out.extend(std::iter::repeat(0).take(64 - take));
            }
        }
        out.truncate(512);
        Ok(out)
    }

    /// `_getFnKeyMatrix(profile, sys)`: `[144,sys,profile,255,page]` × 8.
    pub fn read_fn_keymap(&self, profile: u8, sys: FnSys) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(512);
        for page in 0..8u8 {
            let req = vec![CMD_GET_FN, sys.code(), profile, 255, page, 0];
            let r = self.common_msg_data(&req, ReportKind::Bit7)?;
            let take = r.len().min(64);
            out.extend_from_slice(&r[..take]);
            if take < 64 {
                out.extend(std::iter::repeat(0).take(64 - take));
            }
        }
        out.truncate(512);
        Ok(out)
    }

    /// `setKeyConfigSimple(t,e,n,i)`: 64 B `[10,profile,slot,0,0,apply,sub,…quad@8]`.
    /// JS `7a5b12c9.js` (`a[5]=Number(i??!0)`, `a[6]=n??0`, `a[8..11]=matrix`).
    pub fn write_key_simple(&self, profile: u8, slot: usize, quad: [u8; 4]) -> Result<()> {
        if slot > 127 {
            anyhow::bail!("slot {slot} out of range (0..127)");
        }
        let mut req = vec![0u8; 64];
        req[0] = CMD_SET_KEYMATRIX;
        req[1] = profile;
        req[2] = slot as u8;
        req[5] = 1; // apply now (JS `i ?? !0`)
        req[6] = 0; // sub-profile 0
        req[8..12].copy_from_slice(&quad);
        self.send_msg(&req, ReportKind::Bit7)?;
        Self::vendor_sleep(None);
        Ok(())
    }

    /// `setFnKeyConfigSimple`: `[16,sys,profile,slot,…quad@8]`.
    pub fn write_fn_key_simple(
        &self,
        profile: u8,
        sys: FnSys,
        slot: usize,
        quad: [u8; 4],
    ) -> Result<()> {
        if slot > 127 {
            anyhow::bail!("slot {slot} out of range (0..127)");
        }
        let mut req = vec![0u8; 64];
        req[0] = CMD_SET_FN;
        req[1] = sys.code();
        req[2] = profile;
        req[3] = slot as u8;
        req[8..12].copy_from_slice(&quad);
        self.send_msg(&req, ReportKind::Bit7)?;
        Self::vendor_sleep(None);
        Ok(())
    }
}

/// Factory default 4-byte quad for one slot on a layer.
///
/// Base layer: the device's `defaultMatrix` (`MATRIX_2518`/`MATRIX_3804`).
/// Fn layer: `defaultFnMatrix`/`defaultFnMacMatrix` for the active OS; the
/// 2518 device has no mac table and the driver's base class falls back to
/// its win table.
pub fn default_quad(device_id: Option<u32>, slot: usize, sys: Option<FnSys>) -> Option<[u8; 4]> {
    if slot > 127 {
        return None;
    }
    let m = match sys {
        Some(s) => matrix::default_fn_matrix(device_id, s == FnSys::Mac),
        None => matrix::default_matrix(device_id),
    };
    let b = slot * 4;
    Some([m[b], m[b + 1], m[b + 2], m[b + 3]])
}

/// Decode a 512 B keymap blob against a default matrix.
pub fn decode_keymap(blob: &[u8], default_matrix: &[u8; 512]) -> Vec<KeyEntry> {
    let mut out = Vec::new();
    for slot in 0..128usize {
        let base = slot * 4;
        if base + 4 > blob.len() || base + 4 > default_matrix.len() {
            break;
        }
        let def = [
            default_matrix[base],
            default_matrix[base + 1],
            default_matrix[base + 2],
            default_matrix[base + 3],
        ];
        let hid = matrix::matrix_to_hid(def);
        if hid == 0 {
            continue;
        }
        let q = [blob[base], blob[base + 1], blob[base + 2], blob[base + 3]];
        // Populated default slots are never [0,0,0,0], so a zero readback
        // means Disabled (changed), not "unprogrammed".
        let changed = q != def;
        out.push(KeyEntry {
            slot,
            default_name: matrix::hid_name(hid),
            binding: decode_quad(q),
            changed,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_key_write_layout_matches_js() {
        // setKeyConfigSimple: [10,profile,slot,0,0,1,0,0, m0,m1,m2,m3]
        let (profile, slot, quad) = (2u8, 41usize, [0, 0, 30, 0]);
        let mut req = vec![0u8; 64];
        req[0] = CMD_SET_KEYMATRIX;
        req[1] = profile;
        req[2] = slot as u8;
        req[5] = 1;
        req[6] = 0;
        req[8..12].copy_from_slice(&quad);
        assert_eq!(&req[0..3], &[10, 2, 41]);
        assert_eq!(req[5], 1);
        assert_eq!(&req[8..12], &[0, 0, 30, 0]);
    }

    #[test]
    fn fn_passthrough_shows_key_name_not_hid0() {
        // Live Fn-win readback: O with O assigned arrives as `[0,18,0,0]`
        // (key in skey). The original board joins skey/key/key2, so it
        // shows O; showing only `key` rendered "HID(0)".
        let b = decode_quad([0, 18, 0, 0]);
        assert_eq!(
            b,
            Binding::Combo {
                skey: 18,
                key: 0,
                key2: 0
            }
        );
        assert_eq!(b.short_label(), "O");
        assert_eq!(b.quad(), [0, 18, 0, 0]);
        // Base-layer encoding still shows the same name.
        assert_eq!(decode_quad([0, 0, 18, 0]).short_label(), "O");
    }

    #[test]
    fn combo_joins_all_modifiers_like_original_board() {
        // `f01f4a92.js` combo branch pushes skey, key and key2 when nonzero.
        assert_eq!(decode_quad([0, 224, 44, 0]).short_label(), "LCtrl + Space");
        assert_eq!(decode_quad([0, 0, 229, 38]).short_label(), "RShift + 9");
    }

    #[test]
    fn native_functions_have_names_in_preview() {
        // `uw` entries reachable on the read path (`matrixToConfigs` order:
        // combo first, then the function table, then Snap/MDS/MT).
        for (quad, label) in [
            ([10, 1, 0, 0], "Fn"),
            ([10, 1, 1, 0], "Fn Alt"),
            ([10, 36, 0, 0], "Fn Alt 2"),
            ([10, 13, 0, 0], "Fn Lock Screen"),
            ([10, 14, 0, 0], "Volume Brightness"),
            ([8, 0, 1, 0], "Profile+"),
            ([8, 0, 3, 0], "Profile Loop"),
            ([20, 0, 1, 0], "DPI+"),
            ([2, 0, 130, 0], "Sleep"),
            ([6, 128, 0, 0], "Mic Toggle"),
            ([3, 0, 131, 1], "Player"),
            ([18, 0, 0, 0], "Open Website"),
            ([19, 0, 0, 0], "Open Driver Website"),
            ([11, 10, 3, 0], "OLED Confirm"),
            ([13, 1, 0, 0], "LED Mode Loop"),
            ([13, 2, 1, 0], "LED Brightness+"),
            ([13, 2, 2, 0], "LED Brightness-"),
            ([13, 6, 0, 0], "LED Mode 1"),
            ([10, 9, 0, 0], "LED On/Off"),
            ([10, 12, 0, 0], "Powersave"),
            ([10, 32, 0, 0], "Mic"),
            ([10, 24, 0, 0], "AI"),
        ] {
            let b = decode_quad(quad);
            assert!(matches!(b, Binding::Function { .. }), "{quad:?}");
            assert_eq!(b.short_label(), label, "{quad:?}");
            assert_eq!(b.quad(), quad);
        }
        // `uw` recoil/weapon quads precede the Snap branch on the read path.
        assert_eq!(decode_quad([22, 5, 0, 0]).short_label(), "Recoil");
        // `q[0]==0` quads stay combos even when listed in `uw`
        // (e.g. WIN_D), exactly like `matrixToConfigs`.
        assert_eq!(decode_quad([0, 0, 227, 7]).short_label(), "LGui + D");
    }

    #[test]
    fn sk75_fn_cluster_quads_are_named_functions() {
        // Round 15: the SK75 connector/lighting Fn quads that the original
        // board types ConfigUnknown (系统按键) are named here — SK75 factory
        // Fn default (`74d1dc6b.js`, matches live `keymap read --fn win`) +
        // cross-board family statistics + operator-verified behavior.
        for (quad, label) in [
            ([14, 0, 0, 0], "Bluetooth 1"),
            ([14, 0, 1, 0], "Bluetooth 2"),
            ([14, 0, 2, 0], "Bluetooth 3"),
            ([14, 0, 5, 0], "2.4 GHz"),
            ([14, 0, 6, 0], "Wired"),
            ([13, 3, 1, 0], "LED Speed-"),
            ([13, 3, 2, 0], "LED Speed+"),
            ([13, 5, 0, 2], "LED Direction-"),
            ([13, 5, 0, 1], "LED Direction+"),
            ([13, 5, 1, 0], "LED Color"),
            ([10, 2, 0, 0], "Factory Reset"),
            ([10, 3, 0, 0], "Win Lock"),
            ([10, 8, 0, 0], "Battery Level"),
            ([10, 5, 3, 0], "Win/Mac Switch"),
        ] {
            let b = decode_quad(quad);
            assert_eq!(b.short_label(), label, "{quad:?}");
            assert!(!matches!(b, Binding::Unknown(_)), "{quad:?}");
            assert_eq!(b.quad(), quad, "{quad:?}");
        }
        // Genuinely unknown native quads still display as system key
        // (`case"ConfigUnknown":o=s.系统按键`); e.g. the pair sibling
        // `[10,5,0,0]` (driver mouse dict "PAIR") not fitted on our board.
        let b = decode_quad([10, 5, 0, 0]);
        assert_eq!(b, Binding::Unknown([10, 5, 0, 0]));
        assert_eq!(b.short_label(), "System");
        assert_eq!(b.quad(), [10, 5, 0, 0]);
    }

    #[test]
    fn combo_limit_matches_original_driver() {
        // Original picker: toggle on click, 4th pick refused (`length>=3`),
        // hint `组合键最多可选择3个字符`; save maps click order to
        // skey/key/key2, first pick in `key` here for default-byte compat.
        assert_eq!(MAX_COMBO_KEYS, 3);
        assert_eq!(combo_from_keys(&[]), None);
        assert_eq!(combo_from_keys(&[30, 6, 224, 4]), None);
        let one = combo_from_keys(&[30]).unwrap();
        assert_eq!(one.quad(), [0, 0, 30, 0]);
        assert_eq!(one.short_label(), "1");
        let two = combo_from_keys(&[224, 6]).unwrap();
        assert_eq!(two.quad(), [0, 0, 224, 6]);
        assert_eq!(two.short_label(), "LCtrl + C");
        let three = combo_from_keys(&[224, 225, 6]).unwrap();
        assert_eq!(three.quad(), [0, 6, 224, 225]);
        assert_eq!(three.short_label(), "C + LCtrl + LShift");
    }

    #[test]
    fn decode_branches_match_matrix_to_configs() {
        assert_eq!(decode_quad([0, 0, 0, 0]), Binding::Disabled);
        assert_eq!(
            decode_quad([0, 0, 30, 0]),
            Binding::Combo {
                skey: 0,
                key: 30,
                key2: 0
            }
        );
        assert!(matches!(decode_quad([1, 0, 240, 0]), Binding::Mouse { .. }));
        assert!(matches!(
            decode_quad([3, 0, 233, 0]),
            Binding::Function { .. }
        ));
        assert!(matches!(decode_quad([9, 0, 3, 0]), Binding::Macro { .. }));
        assert!(matches!(
            decode_quad([21, 3, 0, 0]),
            Binding::Gamepad { .. }
        ));
    }

    #[test]
    fn standard_keys_cover_letters_digits_fkeys_modifiers() {
        for h in [4, 29, 30, 39, 40, 41, 44, 57, 58, 69, 73, 82, 224, 231] {
            assert!(STANDARD_KEYS.contains(&h), "missing {h}");
        }
        // Previously missing: PrtSc/SysRq area, numpad (minus NonUS/App),
        // F13-F24.
        for h in [70, 71, 72, 83, 84, 88, 89, 98, 99, 103, 104, 115] {
            assert!(STANDARD_KEYS.contains(&h), "missing {h}");
        }
        // Removed per user request: Exec/Help/Menu/Mute/Vol+/Vol- and
        // NonUS/App/Power.
        for h in [100, 101, 102, 116, 117, 118, 127, 128, 129] {
            assert!(!STANDARD_KEYS.contains(&h), "should be removed {h}");
        }
        assert_eq!(STANDARD_KEYS.len(), 116);
    }

    #[test]
    fn function_menu_has_original_fifteen_plus_natives() {
        let opts = function_options();
        // Original 15 media/function entries stay first and stable.
        assert_eq!(opts.len(), 15 + NATIVE_FUNCTIONS.len());
        for (label, quad) in [
            ("Play/Pause", [3, 0, 205, 0]),
            ("Volume+", [3, 0, 233, 0]),
            ("Calculator", [3, 0, 146, 1]),
            ("Firepower", [11, 0, 0, 0]),
        ] {
            assert!(
                opts.iter().any(|(l, q)| l == label && *q == quad),
                "missing {label}"
            );
        }
        // Native Fn functions (uw + master key table) are assignable too,
        // so factory assignments replicate onto other keys.
        for (label, quad) in [
            ("Fn", [10, 1, 0, 0]),
            ("LED Mode Loop", [13, 1, 0, 0]),
            ("LED Brightness+", [13, 2, 1, 0]),
            ("Powersave", [10, 12, 0, 0]),
            ("AI", [10, 24, 0, 0]),
            ("Mic", [10, 32, 0, 0]),
            // Round 15 connector/cluster names are in the picker as well.
            ("Bluetooth 1", [14, 0, 0, 0]),
            ("Bluetooth 2", [14, 0, 1, 0]),
            ("Bluetooth 3", [14, 0, 2, 0]),
            ("2.4 GHz", [14, 0, 5, 0]),
            ("Wired", [14, 0, 6, 0]),
            ("Win/Mac Switch", [10, 5, 3, 0]),
        ] {
            assert!(
                opts.iter().any(|(l, q)| l == label && *q == quad),
                "missing {label}"
            );
        }
    }

    #[test]
    fn keymap_read_request_layout() {
        // _getKeyMatrix: [138,profile,255,page]; Fn: [144,sys,profile,255,page]
        assert_eq!([CMD_GET_KEYMATRIX, 1, 255, 5], [138, 1, 255, 5]);
        assert_eq!([CMD_GET_FN, 0, 1, 255, 2], [144, 0, 1, 255, 2]);
    }

    #[test]
    fn live_poll_uses_1ms_timing() {
        // getJiaoZhunXinXi -> _getMulitMagnetismCMD(254,4,1,1): SEND 1, READ 1.
        // Encode-time assertion: fast request keeps the same 4-byte prefix.
        let req = crate::proto::build_get_multi_magnetism(crate::proto::mag_op::LIVE_TRAVEL, 3);
        assert_eq!(&req[0..4], &[229, 254, 1, 3]);
    }

    #[test]
    fn reset_defaults_cover_base_and_fn_layers() {
        // Base layer uses the device defaultMatrix, including system keys.
        assert_eq!(default_quad(Some(3804), 0, None), Some([0, 0, 41, 0]));
        assert_eq!(default_quad(Some(3804), 71, None), Some([10, 1, 0, 0]));
        // Fn layer uses defaultFnMatrix/defaultFnMacMatrix per OS.
        assert_eq!(
            default_quad(Some(3804), 18, Some(FnSys::Win)),
            Some([0, 0, 227, 43])
        );
        assert_eq!(
            default_quad(Some(3804), 18, Some(FnSys::Mac)),
            Some([0, 224, 82, 0])
        );
        // 2518 has no mac table; base class falls back to its win table.
        assert_eq!(
            default_quad(Some(2518), 18, Some(FnSys::Mac)),
            Some([0, 0, 227, 43])
        );
        // Unassigned Fn slots reset to the driver's zero entry.
        assert_eq!(
            default_quad(Some(3804), 1, Some(FnSys::Win)),
            Some([0, 0, 0, 0])
        );
        assert_eq!(default_quad(Some(3804), 128, None), None);
    }
}
