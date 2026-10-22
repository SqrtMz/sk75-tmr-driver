# sk75-tmr — WOMIER SK75 TMR Linux driver (Rust)

Rust recreation of the Windows `womier-driver.exe` (WOMIER Driver 3.2.15)
for the **SK75 TMR** analog keyboard, targeting Linux via raw HID feature
reports. It recognizes the per-switch **TMR analog travel values**
(live press depth in mm) without Windows / Electron / `iot_manager`.

## Hardware

| field | value | source |
|---|---|---|
| VID | `0x3151` (12625) | `dist/js/index.5627ed60.js` device DB `VT` |
| PID | `0x5030` (20528) | same (`ry5088_womiersk75he_8k_8k` id 2518, `ry5088_womier_sk75he_europe_3m_8k_8k` id 3804, both `displayName:"SK75 TMR"`) |
| Control interface | iface 2, usage `0x02`, usage page `0xFFFF`, **64-byte Feature, no report ID** | live `lsusb -d 3151:5030`, `/sys/class/hidraw/hidraw3/device/report_descriptor` = `06 FF FF 09 02 A1 01 09 02 15 80 25 7F 95 40 75 08 B1 02 C0` |
| Boot keyboard | iface 0 (hidraw0, 59 B descriptor) | same |
| Composite | iface 1 (hidraw2, 171 B, reports 1/2/3/6 + vendor input report 5/31 B) | same |

Install udev so the hidraw nodes are user-accessible:

```sh
sudo cp udev/99-sk75-tmr.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger
# replug in wired mode afterwards
```

## Protocol (reverse-engineered with REA)

REA on the shipped artifact (see `PROTOCOL.md` for evidence IDs):

- `womier-driver.exe` (`2a01ddb1…`, 102 MiB) is an NSIS-3 stub
  (`binary_overview`: 105 procedures). Payload `$PLUGINSDIR/app-32.7z`
  (105 MiB → 295 MiB) holds Electron `WOMIER Driver.exe` +
  `resources/app` (1138 JS files, `analyze_javascript_application`
  `ev_ccb640c4…`).
- HID lives in `dist/js/7a5b12c9.js` (`class at`, `FEA_CMD_*`),
  `dist/js/60ee4367.js` (magnetism/precision), `dist/js/index.5627ed60.js`
  (transport `ki.Bit7=0/Bit8=1`, `Wi=sum`, `___encodeCmd`, `SEND_TIME=10`,
  `READ_TIME=10`, `defaultVendorSleepTime=100`, device DB),
  matrices `7de29f04.js` / `74d1dc6b.js` (512 B = 128×4 B),
  plus Rust helpers `iot_manager/common_hid_rs.exe`
  (`hid.HidService/SendFeatureReport/GetFeatureReport`, hidapi-rs,
  gRPC `127.0.0.1:3838`, `inspect_rust_binary`, `inspect_pe_resources`).

Transport:

- `ReportKind::Bit7=0`: pad to ≥9 B, `buf[7] = 255-(sum(buf[0..7])&255)`.
- `ReportKind::Bit8=1`: `buf[8] = 255-(sum(buf[0..8])&255)`.
- `sendMsg` → HID Feature write (report ID 0 + up to 64 B, padded to 64 B
  here), `readMsg` → Feature read, `commonMsg` = write then read.
- Keyboard commands use `Bit7` (LED uses `Bit8`).

Analog (TMR) commands:

| cmd | value | meaning |
|---|---|---|
| `SET_MAGNETISM_REPOR` | 27 | enable streaming (`[27,on]`) |
| `SET_MAGNETISM_CAL` | 28 | min-travel cal |
| `SET_MAGNETISM_MAXIMUM_CALIBRATION` | 30 | max-travel cal |
| `SET_MULTI_MAGNETISM` | 101 | batched per-key config |
| `GET_MULTI_MAGNETISM` | 229 | `req=[229,op,1,page]` (op + 4 pages for live) |
| `GET_FEATURE_LIST` | 230 | `resp=[230,0xAA,precision]` |
| `GET_USB_VERSION` / whoAmI | 143 | `ver=r[8]<<8\|r[7]`, `id=u32 LE r[1..4]` |

Multi-mag ops (`et`): `TRAVEL=0, LIFT=1, RT_PRESS=2, RT_LIFT=3,
DYNAMIC_START=4, MT=5, DEADZONE=6, MODE=7, DKS=8, SNAP=9,
LIVE_TRAVEL=254 (按键按压行程数值), LIVE_DYNAMIC=255 (按键按压动态数值),
AXIS=252, TOP_DEADZONE=251`.

Live analog = `GET_MULTI_MAGNETISM(254, 4 pages)` → 256 B → 128×u16 LE
(`_decodeMultiMagnetismTwoByteList`), one slot per matrix position;
`mm = raw / scale`. Scale: `FEA-230` precision
(`0→100, 1→200, 2→1000`) else version fallback
(`None→10, 768–1279→100, ≥1280→200`). JS `_getJiaoZhunXinXi` returns
`{磁通量:0, 行程值:raw, original:hid, index}` per key — flux is
unpopulated in this firmware, travel is the analog value.

## Usage (UI only)

```sh
cargo build --release
./target/release/sk75-tmr-ui
```

`sk75-tmr-ui` is the only binary. There is no CLI subcommand interface:
device information, live analog values, rebinding, switch selection and
travel calibration are all in the desktop window.

- **Device info tab** — the “read keyboard information” action: VID:PID
  (hex + dec), hidraw path, product/manufacturer, device ID
  (2518 `ry5088_womiersk75he_8k_8k` / 3804 `…_europe_3m_8k_8k`),
  variant, USB version (cmd 143 `r[8]<<8|r[7]`), precision (FEA 230
  `[230,0xAA,p]` → scale 100/200/1000, else version fallback),
  report rate (cmd 131 `e[2]` → 125–8000 Hz) and key count. Transport
  errors are shown inline with the wired-mode + udev hint instead of
  crashing.
- **Keyboard tab** — the 81-key board in the original arrangement
  (positions from `7d1d89fb.js`), plus full rebinding and per-key
  switch selection. Each key shows its original cap name, current
  binding, configured press/release activation travel (red/blue) and,
  with Live input enabled, the current op-254 depth in mm and % of the
  4.0 mm full stroke. Live press depth tints the key fill; reaching the
  configured actuation point turns the fill black with a red glow.
  Clicking a key opens the binding editor below the board: profiles 1–4,
  Base/Fn-win/Fn-mac layers, 116 standard keys, mouse, media/functions,
  native Fn functions, gamepad, macro and disabled. Write/Reset/Write
  switch use the same single-key ops as the original driver. Hovering a
  key shows its slot, binding, travels and live value.
- **Travel Calibration** — the card to the left of the keyboard preview
  starts the original per-key calibration flow. It opens the
  “Keystroke Calibration Operation Guide” and runs the same startup
  sequence non-blockingly: cmd 28 (min-cal) on, 2 s, cmd 28 off,
  cmd 30 (max-cal) on, then a 500 ms settle. The guide's **Start
  Calibration** button is disabled until that sequence finishes.
  After Start, op-254 travel is polled continuously and every key is
  painted with the original `#74EBD5` overlay: partial presses fill
  proportionally at 0.3 opacity; reaching the firmware threshold locks
  the key at 100% height / 0.8 opacity and it stays locked after
  release. **Finish calibration** is enabled only after every
  expected key has locked. Finish sends cmd 30 off and keeps the
  calibrated keys at full travel. Cancel, an error, or closing the
  window always turns min/max calibration off.
  The threshold matches the original UI:
  `version.rf ?? version.usb`, then `y && y < 768 ? 1 : 300`.
  The Fn key is included via the matrix marker `[10,1,0,0]`
  (`167837696`); production variant 3804 expects all 81 on-screen
  keys. Variant 2518 maps 80 because its matrix has RCtrl where the
  shared UI geometry has AltRight.
- **On-screen board preview** — ~50% of the screen width (responsive),
  each key filled with its installed switch's color from the driver's
  `轴体颜色字典`; the legend below maps the colors. The demo's corner
  switch-number badge stays off. Press/release travels are shown at the
  bottom of each key, separated from the cap and binding by a spacer
  band. During calibration, normal live-depth tint/glow is suppressed
  so the calibration overlay is unambiguous.

Keep the keyboard **wired** for calibration and live reads. The original
JS loop and this UI both require the vendor feature-report interface
(iface 2 / usage page `0xFFFF`).

## Layout

- `src/proto.rs` — checksum, request builders, LE decode, mm conversion.
- `src/matrix.rs` — both SK75 base matrices, Fn win/mac default matrices,
  HID names.
- `src/hid.rs` — hidapi transport + high-level ops (`report_rate_hz` added).
- `src/device_info.rs` — consolidated info readout used by the UI.
- `src/layout.rs` — 81-key on-screen geometry from the original asset and
  HID→matrix-slot mapping (including the Fn marker).
- `src/switches.rs` — SK75 switch table (driver-named) + colors + op-252
  read/write (code at byte 8).
- `src/keymap.rs` — rebinding: `read_keymap`/`read_fn_keymap` (cmds
  138/144), `write_key_simple`/`write_fn_key_simple` (cmds 10/16),
  `default_quad` (Base/Fn Reset defaults),
  `Binding` options (Key/Disabled/Mouse/Media/Gamepad/Macro/Special),
  profiles 1–4 + Fn win/mac.
- `src/ui.rs` — UI entry point and egui implementation (`sk75-tmr-ui`:
  Device info / Keyboard, including the travel-calibration guide,
  startup state machine and key overlay).
- `udev/99-sk75-tmr.rules` — permissions.

## Limitations

- Static RE only: no firmware execution; 2.4 M AST nodes partially
  covered, dynamic IPC unresolved (see REA limitations).
- `hidapi` device enumeration duplicates hidraw2 collections; opener
  prefers iface 2 / page `0xFFFF` (hidraw3).
- Wireless (2.4 G/BT) dongle framing (`246/247/252/254`, `85` prefix on BT)
  is not implemented — USB only.
- No LED or macro-content write paths yet (remap + switch writes are
  implemented; read the JS `setMultiMagnetismInfo` chunking in
  `PROTOCOL.md` to extend).
- Travel calibration is wired-only. It uses the original cmd 28/30
  sequence and op-254 threshold; the firmware owns persistence.
- Variant 2518 cannot match the shared 81-key UI geometry one-to-one
  (RCtrl in its matrix vs AltRight in the layout), so calibration expects
  80 slots there. Variant 3804 maps all 81.
