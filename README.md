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

## Usage (CLI)

```sh
cargo build
./target/debug/sk75-tmr list
./target/debug/sk75-tmr info
./target/debug/sk75-tmr monitor --seconds 5
./target/debug/sk75-tmr monitor --seconds 3 --dynamic --all
./target/debug/sk75-tmr calibrate --settle 8
./target/debug/sk75-tmr stream --on true
./target/debug/sk75-tmr demo   # offline analog-decode proof, no hardware
```

`monitor` prints `slot hid name raw mm [dyn]` for every pressed TMR
switch at max HID-bound speed (~83 Hz, 1 ms pacing, no throttle). Keep the keyboard **wired**
(wireless calibration/reads are slower and the JS loop refuses BT).

## UI (`sk75-tmr-ui`)

Native Linux desktop recreation of the driver window
(`dist/index.html` title `GearHub`, device card in `dist/js/f70eb088.js`
with `固件版本：{h}` + firmware-upgrade button and `IOT版本：{a}` +
check-update button):

```sh
cargo run --bin sk75-tmr-ui
```

```sh
# CLI rebinding (mirrors 改键 + Fn layers; single-key writes)
./target/debug/sk75-tmr keymap read --profile 0
./target/debug/sk75-tmr keymap read --profile 0 --fn win
./target/debug/sk75-tmr keymap write --profile 0 --slot 0 --hid 30
./target/debug/sk75-tmr keymap write --profile 0 --slot 0 --disable
./target/debug/sk75-tmr keymap write --profile 0 --slot 5 --mouse Left
./target/debug/sk75-tmr keymap write --profile 0 --slot 5 --function 'Volume+'
# Live poll always runs at max speed like the original loop
./target/debug/sk75-tmr monitor --seconds 2   # → ~83 Hz measured
```

- **Device info tab** — the “read keyboard information” option: VID:PID
  (hex + dec), hidraw path, product/manufacturer, device ID
  (2518 `ry5088_womiersk75he_8k_8k` / 3804 `…_europe_3m_8k_8k`),
  variant, USB version (cmd 143 `r[8]<<8|r[7]`), precision (FEA 230
  `[230,0xAA,p]` → scale 100/200/1000, else version fallback),
  report rate (cmd 131 `e[2]` → 125–8000 Hz), key count. Transport
  errors are shown inline with the wired-mode + udev hint instead of
  crashing. Live unit on this host reads e.g. id `3804`, USB `768
  (0x0300)`, scale `100`, `8000 Hz`, `81` keys.
- **Analog tab** — live TMR bars from op 254 (`op 255` optional):
  per-switch `raw` + `mm`, threshold filter, measured-Hz readout.
  Always max speed like the original `getJiaoZhunXinXiLoop`: 1 ms HID
  pacing, back-to-back polls, no throttle (~83 Hz on real hardware).
- **Keyboard tab** (combined board + rebinding, live input included, so
  there is no separate Analog tab) — on-screen 81-key board in the
  original arrangement (positions from its keymap asset): every key
  visible at once with its label, current binding and live press depth.
  Clicking a key opens its rebind options right below: profile 1–4 and
  Base/Fn-win/Fn-mac layer selectors (changing either re-reads the
  keyboard configuration, mirroring `setCurrentProfile` cmd 4),
  at-a-glance option grids (116 standard keys as buttons from the
  driver's key tables — letters, digits, PrtSc/Scroll/Pause, numpad,
  F13–F24, modifiers; click adds / re-click removes, up to 3 per combo
  mirroring the original `handleClick_组合键` limit; arrow keys shown as
  Nerd-Font icons loaded from
  the system fonts; no HID numbers to type — plus mouse, the original
  15 media/function entries, gamepad, macro and disabled; Write key
  auto re-reads and keeps the key selected), per-key
  switch type across the six SK75 options (op 252), and Write key /
  Write switch actions.
- **Calibration tab** — travel calibration buttons (cmds 28/30,
  same min→max sequence as `行程校准`).

## Layout

- `src/proto.rs` — checksum, request builders, LE decode, mm conversion.
- `src/matrix.rs` — both SK75 matrices + HID names.
- `src/hid.rs` — hidapi transport + high-level ops (`report_rate_hz` added).
- `src/device_info.rs` — consolidated info readout shared by CLI + UI.
- `src/main.rs` — CLI (`list/info/monitor/keymap/switch/calibrate/stream/demo`).
- `src/layout.rs` — 81-key on-screen geometry from the original asset.
- `src/switches.rs` — SK75 switch table + op-252 read/write.
- `src/keymap.rs` — rebinding: `read_keymap`/`read_fn_keymap` (cmds
  138/144), `write_key_simple`/`write_fn_key_simple` (cmds 10/16),
  `Binding` options (Key/Disabled/Mouse/Media/Gamepad/Macro/Special),
  profiles 1–4 + Fn win/mac.
- `src/ui.rs` — egui UI (`sk75-tmr-ui`: Device info / Keyboard / Calibration).
- `udev/99-sk75-tmr.rules` — permissions.

## Limitations

- Static RE only: no firmware execution; 2.4 M AST nodes partially
  covered, dynamic IPC unresolved (see REA limitations).
- `hidapi` device enumeration duplicates hidraw2 collections; opener
  prefers iface 2 / page `0xFFFF` (hidraw3).
- Wireless (2.4 G/BT) dongle framing (`246/247/252/254`, `85` prefix on BT)
  is not implemented — USB only.
- No LED/macro/remap write paths beyond analog + calibration (read the
  JS `setMultiMagnetismInfo` chunking in `PROTOCOL.md` to extend).
