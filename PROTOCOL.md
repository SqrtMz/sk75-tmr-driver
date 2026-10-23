# SK75 TMR protocol notes (REA-derived)

> **Current entry point (2026-10-22): UI only.** `src/main.rs`, the
> `sk75-tmr` binary and `clap` were removed. Historical `sk75-tmr …`
> examples later in this log record earlier iterations and are not
> current commands. The GUI is `sk75-tmr-ui`; calibration now lives in
> the Keyboard tab, to the left of the board.

## Artifacts

- `/home/mz/repositories/womier-test/womier-driver.exe`
  sha256 `2a01ddb1f7a1d4c7d75703b07d188c85bece74913bab39e67494378e55116826`,
  105996280 B, `PE32 i386, NSIS self-extracting, 5 sections`
  (`7z l`: `NSIS-3 Unicode`, `$PLUGINSDIR/app-32.7z` 105497564 B).
- `$PLUGINSDIR/app-32.7z` → 1360 files, 295182484 B → `app/`:
  Electron (`WOMIER Driver.exe` 130 MiB), `resources/app`
  (`package.json` `WOMIER Driver 3.2.15`, `APPVersion.json` IOT 222,
  `dist-electron/main.js`, `iot_manager/*.exe`, `dist/js/*.js`).
- `keyboard-firmware-installer.exe` (`f148c32a…`, 20 MiB, `ry_upgrade`)
  is the separate firmware updater, not the driver. REA
  `inspect_artifact` (`ev_f7fea484…`) + `inspect_rust_binary`
  (`ev_24dd88d0…`) + `inspect_pe_resources` (`ev_e3099370…`) identify a
  PE32 i386 Rust/Slint build with no `.dep-v0` (MSVC, not auditable) and
  only an icon resource. Strings (shell corroboration) expose
  `src/store/hall_store.rs` callbacks `on_set_travel` and
  `on_calibrate_travel` plus `on_start_report`/`on_stop_report`, and
  upgrade modules `keyboard8k.rs`, `ry5088.rs`, `ch585ms.rs`. Native
  decompilation of `on_set_travel` was blocked: Hopper's bridge timed out
  waiting for readiness, and Ghidra/IDA/rizin are unconfigured. The
  installer therefore corroborates the travel/calibration feature naming
  but does not provide a second wire-level implementation; the write
  semantics below come from the shipped driver JS.

## REA calls (this session)

1. `open_binary(womier-driver.exe)` → PE x86, ok.
2. `inspect_artifact{detail:summary}` → 1 node, no children
   (NSIS payload opaque to the graph provider; extracted with `7z`).
3. `binary_overview` (Hopper) → 7 segments, 105 procedures, 38 strings.
4. `binary_session{}` → Hopper bound, capabilities inventoried.
5. `inspect_rust_binary(common_hid_rs.exe)` → PE amd64 3073600 B,
   no `.dep-v0` (expected: Windows MSVC build, not auditable).
   Same for `iot_manager_rs.exe` (6534720 B), `keyboard_monitor_service.exe`,
   `rhythm_service.exe`, `system_info_hid_rs.exe`.
6. `analyze_javascript_application(resources/app)` → retained
   `ev_ccb640c41ec213061a7afdd7f27a3b5386df1c06ba5ea09af268c7c00885a016`
   (1138 files, 2441029 AST nodes, 8389 findings, partial coverage) +
   `inspect_analysis_view(summary)` →
   `ev_f5d90268a497fcdbae4ba35e7d09350dc9de30cb99f801cbc9b83a745e9bbe81`.
7. `close_binary` + `open_binary(common_hid_rs.exe)` → Hopper timeout
   (120 s, x86_64 PE); fell back to target-free tools:
8. `inspect_managed_artifact(common_hid_rs.exe)` → `not-managed`
   (`cli-directory-absent`), `inspect_pe_resources` → PE32+, 0 resources.
9. `strings` (shell corroboration, not REA): `hid.HidService/*`,
   `hidapi-rs`, `HidD_SetFeature/GetFeature`, `127.0.0.1:3838`,
   `DeviceFilter{vendor_id,product_id,usage,usage_page,…}`.

## JS map

- `7a5b12c9.js` `class at extends j` — base keyboard:
  `FEA_CMD_*` (SET 1/3/4/6/7/8/9/10/11/12/16/17/18/23/27/28/30/37/40/41/80/101/172,
  GET 128/131/132/133/134/135/136/137/138/139/140/143/144/145/151/229/165/169/208),
  `et` ops 0..10 + 251/252/254/255,
  `sendMsg/commonMsg/vendorSleep`, `_getMulitMagnetismCMD(229)`,
  `_decodeMultiMagnetismTwoByteList`, `setKeyMagnetismReport(27)`,
  `_setZuiXiao/Max…(28/30)`, `_getJiaoZhunXinXi(254)` →
  `{磁通量:0,行程值:u16,original:hid,index}`, `getJiaoZhunXinXiLoop`,
  `setJiaoZhunKaiGuan`, `get磁轴行程步进倍数` (version rule),
  `get支持顶部死区` (rf>=1024||usb>=1024).
- `60ee4367.js` `class H extends q` — `GET_FEATURE_LIST=230`
  (`[230,0xAA,precision]`), `Precision001=0/0005=1/0001=2`
  (`index.js am`), scale 100/200/1000, `setMultiMagnetismInfo`.
- `index.5627ed60.js` — `ki{Bit7:0,Bit8:1,None:2}`,
  `Wi=sum`, `___encodeCmd` (pad 9, checksum at 7/8),
  `pK{SEND_TIME:10,READ_TIME:10,defaultVendorSleepTime:100}`,
  `cK` vendor events (`27→磁轴行程`), `Qf` WebHID filters
  (vendor `usage:2,usagePage:65535,interfaceNumber:2`),
  `VT/DT` device DB incl. SK75 rows below, `or{hidToMatrix,matrixToHid}`,
  gRPC-Web `iot_manager.*` + `hid.HidService`.
- Devices:
  `{id:2518,vid:12625,pid:20528,keyLayout:Common81_AGK75B,
  name:"ry5088_womiersk75he_8k_8k",displayName:"SK75 TMR",company:"WOMIER",
  magnetism:true, …supportedSwitchTypes:["高特","磁玉pro","磁玉gaming",
  "万磁王RGB","万磁王POM","玄磁轴"]}`,
  `{id:3804,vid:12625,pid:20528,keyLayout:Common81_SK75TMR,
  name:"ry5088_womier_sk75he_europe_3m_8k_8k",displayName:"SK75 TMR",…same}`.
- Matrices: `7de29f04.js` (2518), `74d1dc6b.js` (3804), 512 B each,
  differ at slots 65/67 only. UI map `7d1d89fb.js`
  (`keyboard_81_sk75tmr_keymappings_ui_info`, 81 keys).
- Helpers: `common_hid_rs.exe` (hidapi `Send/GetFeatureReport`,
  `HidD_*`, `hid_open_path`, `127.0.0.1:3838`),
  `iot_manager_rs.exe` (`IotManager/*`, `AddDeviceFilter`,
  `IotDeviceFilter`, `127.0.0.1:3838/3839/3840/3850/6015`).

## Live hardware (this Linux host)

- `lsusb -d 3151:5030` → `Bus 005 Device 002: ID 3151:5030 SK75 TMR`,
  3 interfaces (0 boot kbd, 1 composite, 2 vendor).
- `hidraw3` descriptor (20 B):
  `06 FF FF 09 02 A1 01 09 02 15 80 25 7F 95 40 75 08 B1 02 C0`
  = vendor page `0xFFFF`, usage `0x02`, **64×u8 Feature** (`B1 02`),
  no report ID → wire format is report-ID 0 + 64 B.
- `hidraw2` (171 B) ends with vendor *input* `85 05 … 95 1F`
  (report 5, 31 B) — not the control path.
- `sk75-tmr list` enumerates all three nodes + collections and the
  opener prefers `iface 2 / page 0xFFFF` (hidraw3).

## UI expansion (this request)

- REA `analyze_javascript_application` (scoped: `7a5b12c9.js`,
  `60ee4367.js`, `7d1d89fb.js`, `7de29f04.js`, `74d1dc6b.js`) →
  parent `ev_275ba67b…`, summary view `ev_a9d5046b…`
  (5 files, 20087 AST nodes, 53 findings; 1140 files out of scope).
- REA `inspect_analysis_view` module page → `ev_43d23e55…`
  (20/1149 modules); module item `dist/js/7a5b12c9.js` →
  `ev_ccc7a2e4…` (minified bundle, no declared exports — expected).
- `trace_application_feature` was attempted for `getUSBVersion` but the
  tool's `application` parameter does not accept a plain evidence-ID
  reference in this build; device-info flow was instead corroborated by
  direct static reads: `f70eb088.js` card rows `固件版本：{h}` +
  upgrade button / `IOT版本：{a}` + update button, i18n mirrors in
  `efde418b.js` / `ad612a2f.js`, `APPVersion.json` (3.2.15/IOT 222),
  `company_WOMIER/CONFIG.json`, `index.html` title `GearHub`.
- Device-info option implemented in `src/device_info.rs`
  (`read_keyboard_info`, never-panics error capture) and surfaced in
  both `src/main.rs` (`info`) and `src/ui.rs` (Device info tab).
  Live readout on attached unit: id 3804, USB 768, scale 100, 8000 Hz.

## Rebinding + max-speed poll (this request)

- REA `analyze_javascript_application` (scoped `7a5b12c9.js` +
  `15394fe0.js` + `ec0c0cb4.js`) → parent `ev_a8a11954…`, summary
  `ev_942b38b6…` (3 files, 16070 AST nodes, 45 findings); module page
  `ev_4e41ffbf…`.
- Keymap wire format (`7a5b12c9.js` `class at`): read
  `_getKeyMatrix` = `[138,profile,255,page]` × 8 pages,
  `setKeyConfigSimple` = 64 B `[10,profile,slot,0,0,1,0,0, m0..m3@8]`;
  Fn: `_getFnKeyMatrix` = `[144,sys,profile,255,page]` × 8
  (`_解码fnSys`: win 0/mac 1/android 2/ios 3),
  `setFnKeyConfigSimple` = `[16,sys,profile,slot,…,quad@8]`;
  profiles 0..3 (`layer:4`), Fn win/mac × 1 (`fnSysLayer`).
- Binding options (`index.5627ed60.js` `matrixToConfigs`/`configToMatrix`/
  `changeArrToConfig`, tables `uw`/`Fa`/`O_`): combo `[0,skey,key,key2]`,
  forbidden `[0,0,0,0]`, mouse `Fa` (Left `[1,0,240,0]`…),
  function `uw` (Volume+ `[3,0,233,0]`… Firepower `[11,0,0,0]`),
  gamepad `O_` (`[21,…]`/`[255,255,255,…]`), macro `[9,type,idx,0]`
  (repeat 0/on_off 1/touch 2), Snap/MDS/MT/recoil `[22|23|24,…]`.
- Live speed (`7a5b12c9.js`): `_getMulitMagnetismCMD(t,e,n=SEND_TIME,
  i=READ_TIME)` defaults 10/10 (`pK`), but `_getJiaoZhunXinXi` calls
  `(254,4,1,1)` and `getJiaoZhunXinXiLoop` re-polls with no sleep.
  Implemented as 1 ms fast path + back-to-back polling; measured
  167 polls/2 s = 83 Hz on the attached SK75 TMR (HID-bound).
- Live verification: `keymap read --profile 0` (defaults intact),
  `--fn win` (real Fn bindings), `monitor --seconds 2` → 83 Hz.
  No layout writes were made to the attached keyboard.

## Switch types, visual board, fast-only poll (this request)

- REA `analyze_javascript_application` (scoped `7a5b12c9.js` +
  `60ee4367.js` + `7d1d89fb.js` + `e796a76e.js`) → parent `ev_d25f8466…`,
  summary `ev_927021fe…` (4 files, 18250 AST nodes, 44 findings);
  module page `ev_6b3387ab…`.
- Switch types: axis table `P` (`Gaote=0, Jade Pro=2, Jade Gaming=3,
  Magnet-King RGB=12, POM Magnet-King=13, Xuan Magnetic=69`, …109 entries),
  SK75 `supportedSwitchTypes` (6 entries, id 2518 + 3804) with special
  display POM mapping; single-key write
  `[101,252,0,slot,1,0,0,ck,code]` (code at byte 8; an earlier build put
  it at byte 5 and the firmware stored 0 — see the 2026-10-22 section),
  batch read `(252,2)`. Live check: slot 8 reads 69, unset slots 0xFF
  (driver default Gaote), trailing slots 0.
- Poll: the 10 Hz cap is removed everywhere; live reads always use the
  original 1 ms pacing back-to-back (measured 83–84 Hz).
- Board: all 81 rects from `7d1d89fb.js` (732×281 canvas), each showing
  label + binding + live mm with depth shading; click selects for remap.
- Language: `src/*.rs` contains zero CJK characters; all user-visible
  strings are English (evidence quotes live only in this file).

## Combined board, picker grids, profile re-read (this request)

- REA `analyze_javascript_application` (scoped `f01f4a92.js` board +
  key tables, `9a894487.js` profile switch, `c0690a03.js` layer state)
  → parent `ev_9da292dd…`, summary `ev_29a44018…` (3 files, 10211 AST
  nodes, 47 findings); module page `ev_aa9d3098…`.
- Click-key-shows-options mirrors the original selection flow
  (`current selected single key` + `keyCodeToDisplayText` picker in
  `f01f4a92.js`); the 83-key option grid covers the driver's combo
  targets (letters, digits, controls, F-keys, navigation, modifiers)
  with mouse/media/gamepad/macro/disabled alongside, all visible at
  once instead of typed HID numbers.
- Profile/layer switching mirrors `setCurrentProfile` (cmd 4,
  `9a894487.js`) + `getCurrentProfile` (cmd 132): changing profile
  1–4 or Base/Fn-win/Fn-mac re-reads keymap (cmds 10/138, Fn 16/144)
  plus switch types (op 252).
- Analog tab removed: the board already overlays live input at the
  same max speed (~81 Hz measured).

## Full key list + named numeric functions (this request)

- REA `analyze_javascript_application` (scoped `f01f4a92.js` picker +
  `9a894487.js` profile switch + `59693644.js`/`7512cac2.js`)
  → parent `ev_0cc79107…`, summary `ev_0d098ffb…` (3 files, 17815 AST
  nodes, 63 findings); module page `ev_1f7d6262…`.
- Standard keys grew 83 → 125: added PrtSc (70), ScrollLock (71),
  Pause (72), NumLock + full numpad (83–99), Non-US/App (100–101),
  Power/Keypad-= (102–103), F13–F24 (104–115), Execute/Help/Menu
  (116–118), Mute/Vol+/Vol- (127–129).
- Numeric-code functions named from the `uw` table
  (`index.5627ed60.js`): Fn `[10,1,0,0]` / Fn-alt `[10,1,1,0]` /
  Fn lock screen `[10,13,0,0]` (shares bytes with a duplicate
  volume-brightness alias, kept as Fn lock), Volume<->Backlight
  `[10,15,0,0]` (+ alt `[10,14,0,0]`, RGB-brightness knob swap),
  Pair `[10,5,0,0]`, Countdown `[10,24,0,0]`, Profile family
  `[8,0,n,m]` (value/+/-/loop/set 1–5, firmware cmd-4 family),
  DPI `[20,0,n,0]`, System power/sleep/wake `[2,0,129..131,0]`,
  OLED `[11,10,n,0]`, Mic/apps `[6,128,0,n]`, website/assistant
  `[18,…]`. All quads unique; decode-tested.

## Pruned picker, reverted functions, nerd-font arrows (this request)

- Standard keys 125 → 119: removed Exec/Help/Menu (116–118) and
  Mute/Vol+/Vol- (127–129) from the rebind picker (decode names kept).
- Function menu reverted to the original 15 media/function entries;
  the extended numeric-code table stays documented above for reference.
- Arrow keys render as Nerd-Font icons (FA codepoints U+F060–F063 via
  `matrix::arrow_glyph`, font auto-loaded from system Nerd Fonts).

## Fn preview names: combo join + System label (this request)

- REA `analyze_javascript_application` on the freshly extracted
  `app-32.7z` tree (`/tmp/opencode/womier-app/resources/app`, scoped
  `7a5b12c9.js`+`f01f4a92.js`+`15394fe0.js`+`ec0c0cb4.js`) →
  `ev_015a49f6a99494fe3c50d39b092ce21c6740ee25d754b524095ba4a5744831a2`
  (1141 files out of scope), corroborated by direct static reads of
  `7a5b12c9.js` (`setFnKeyConfigSimple`, `matrixToConfigValues`,
  `defaultFnMatrix=V`), `index.5627ed60.js` (`matrixToConfigs`,
  `configToMatrix`, `changeArrToConfig`, `uw`/`fL`/`O_`/`Fa` tables) and
  `f01f4a92.js` (board overlay combo + `ConfigUnknown` display branches).
- Read-path order (`matrixToConfigs`): forbidden → macro(9) → mouse(1) →
  gamepad(21) → combo(0) → `uw` function table → 11 → recoil/snap(22) →
  MDS(23) → MT(24) → unknown. So `q[0]==0` quads (e.g. WIN_D
  `[0,0,227,7]`) decode as combos, and `uw` recoil/weapon quads
  (`[22,5,0,0]`, `[22,6,255,0]`, `[22,1..3,0,0]`) decode as functions,
  never Snap. `decode_quad` now mirrors this order.
- Combo display joins every nonzero of skey/key/key2 (`f01f4a92.js`
  combo branch). Fn passthrough stores the key in skey (live: O as
  `[0,18,0,0]`), which previously rendered as `HID(0)`; now shows `O`.
  Picker preselect falls back skey → key2 when key is 0.
- Unmatched native quads are ConfigUnknown, shown on the original board
  as system key (`case"ConfigUnknown":o=s.系统按键`); preview/CLI now
  show `System` instead of `[a,b,c,d]` (live Fn win: `[10,2,0,0]`,
  `[14,0,0,0]`, `[13,5,0,2]` …). Quads are still preserved byte-exact
  for writes (`quad()` round-trips; write path untouched).
- `uw` display names added decode-only (39 entries: media extras,
  power, mic/apps, profiles, Fn, recoil/weapon, DPI, OLED, website);
  the 15-entry rebind picker is unchanged. `fL` countdown
  (`[10,24,0,0]`) deliberately stays System: both original paths type
  it ConfigUnknown.

## Combo multi-key: 3-key limit, toggle picker (this request)

- REA `analyze_javascript_application` (scoped `0482e620.js`+
  `2e81aae0.js`+`59693644.js`) → `ev_749cc9f3bd7aecf6ca89cf8531e23291c9c0333e7c7b5b22fe262d414c7770eb`,
  corroborated by direct static reads of the extracted `app-32.7z` tree.
- Toggle on click (`handleClick_组合键` in `index.5627ed60.js`, same in
  `59693644.js`): present → remove, absent → refuse when full
  (`if(length>=3)return`). Panel hint `组合键最多可选择3个字符`
  ("combo holds at most 3", also in i18n `efde418b.js`/`ad612a2f.js`).
- Save (`设置改键("组合键")`, keyboard + mouse paths) maps click order
  positionally to `skey/key/key2`; empty list refuses
  (`if(list.length===0)return`). Preseed splits current text on `+`.
- Limit reason (observed): combo wire encoding is `[0,skey,key,key2]` —
  one 4-byte keymap slot whose first byte is the combo tag, leaving
  exactly 3 key bytes (`configToMatrix`); longer combos are unencodable.
  Whether firmware would additionally reject them is unobserved (static
  RE only). Single deviation, documented in code: first pick fills `key`
  (not `skey`) so lone keys keep firmware default bytes `[0,0,hid,0]`;
  display is identical via the skey/key/key2 join.
- Implemented: `keymap::MAX_COMBO_KEYS=3` + `combo_from_keys`, UI Key
  picker toggles with Clear + `n/3` readout, CLI `--hid` takes 1–3 HIDs.

## Fn native names from master key table + assignable Functions (this request)

- REA `analyze_javascript_application` (scoped `index.5627ed60.js`+
  `59693644.js`) → `ev_42330295…`, corroborated by direct static reads.
- The 176-entry master key table
  (`{value,scanCode,hidCode,keyCode,htmlCode}`, `hidCode` = big-endian
  quad) names the live Fn quads: `[13,1,0,0]` → `LED_Mode` (shown as
  "LED Mode Loop": same action as the knob-push `LEDMODELOOP`),
  `[13,2,1,0]`/`[13,2,2,0]` → `LEDBIRINC`/`LEDBIRDEC`,
  `[13,6,0,0]`/`[13,6,1,0]` → `ledMode1`/`ledMode2`,
  `[10,12,0,0]` → `Powersave`, `[10,36,0,0]` → `r_Fn`,
  `[10,32,0,0]` → `MIC`, `[10,9,0,0]` → `LEDONOFF`,
  `[10,14,0,0]` → `Vol-Bri`/`Volume_Brightness`,
  `[19,0,0,0]` → `OpenWebsiteDriver`, `[3,1,205,0]` → second
  `MediaPlayPause`. Display helper (`59693644.js` `$`) confirms
  ConfigFunction shows `r.key` while ConfigUnknown shows system key.
- `[10,24,0,0]` labels AI per the master table (supersedes the earlier
  countdown-System note; `fL` still sends the same code for countdown).
- Still honestly unknown (no table entry anywhere in the app):
  `[10,2,0,0]` (Esc), `[10,3,0,0]` (LGui/Win-lock?), `[10,5,3,0]` (M),
  `[10,8,0,0]` (Bksp), `[13,3,x]`/`[13,5,x]` (PgUp/PgDn/Left/Right/End),
  `[14,0,x]` (Q/W/E/R/T) — these stay System, exactly like the original
  board (`ConfigUnknown` → system key).
- `function_options()` is now the 15 originals + all 50 native entries
  (65 total), so every named Fn assignment (incl. Fn itself) replicates
  via the Functions picker, CLI `--function`, and single-key writes;
  bytes round-trip exact. Observed live: Home → LED Mode Loop, Up/Down →
  LED Brightness±, Del → Powersave.
- Note: Fn-win slot 56 (O) read `[0,18,0,0]` during REA but `[0,0,0,0]`
  afterwards; both decode correctly (O passthrough vs Disabled per the
  original's forbidden semantics). Only writes made this session were
  base slot 0 LCtrl + immediate Esc restore (readback verified clean).

## Board preview travel values + per-switch colors + switch names (this request)

- REA `analyze_javascript_application` (scoped `2a8cd9c4.js`+
  `d9f04973.js`+`1eafbcc7.js`+`4f13328e.js`+`9528bb70.js`+
  `index.5627ed60.js`+`60ee4367.js`) →
  `ev_a6632e7b81048a6be876ba8d6c4d399de560fe496f69b34d887e1f1756bb13cf`,
  corroborated by direct static reads.
- Travel values come from `getMultiMagnetismInfo`
  (`7a5b12c9.js`): press `触发行程` = `_getMulitMagnetismCMD(0, 4)`,
  release `抬起行程` = `_getMulitMagnetismCMD(1, 4)` — 4 pages each,
  u16 LE per slot, divided by the travel-step scale for mm.
  On-key rendering (`d9f04973.js` `ve`/`pe` + SVG template
  `KeyBox_KeyMappings_RT` in `2a8cd9c4.js`): press bottom-left
  `#FE4545`, release bottom-right `#0066FF`, font-size
  `8*(4/(precision+2))`, `toFixed(travelPrecision)` formatting;
  `travelPrecision` getter = decimal digits of the press-travel step
  (default 2) → implemented as `proto::travel_digits(scale)`
  (100→2, 200→3, 1000→3).
- Switch colors from `轴体颜色字典` (`index.5627ed60.js`), applied by
  the driver `轴体颜色字典[轴体] ?? 16771625`; unknown code/0xFF falls
  back to 高特 like `_解码轴体`. Our fills (plus a depth-driven
  lighten and darker-than-driver notch, our aesthetic choices):
  高特 16771625 #FFEA29, 磁玉pro 3080153 #2EFFD9,
  磁玉gaming 12026111 #B780FF, 万磁王RGB 15746815 #F046FF,
  万磁王POM 14835949 #E260ED, 玄磁轴 6970061 #6A5ACD.
- Switch names: driver-side English from i18n `9528bb70.js`
  (`高特:"Outemu"`, `磁玉pro:"Magnetic Jade Pro"`,
  `磁玉gaming:"Magnetic Jade Gaming"`, `万磁王RGB:"Magneto RGB"`,
  `万磁王POM:"POM Magnet King"` via SK75
  `specialSwitchDisplayName:{万磁王POM:"POM万磁王"}`,
  `玄磁轴:"Void Switch"`); display
  `{driver name} - {source name}`. Note: request said "POM Manget
  King" → corrected to the driver's "POM Magnet King".
- Board, per request, also shows the switch-number badge
  (`supportedSwitchTypes.findIndex(轴体)+1`, notch top-left) and
  selected-key outline `#FD4444` (rgba(253.94,68.77,68.77), 1.7px).
- Implemented: preview scales to available width (fixed-732px canvas
  clipped labels before), fonts scale with it; key fill = switch color;
  red/blue activation values per key; hover tooltip (binding + travels +
  switch); color legend. New CLI `travel` subcommand prints
  press/release per key. Live check: all 81 mapped keys read
  press 50 / release 50 = 0.50 mm (uniform factory value, ops 0/1);
  switch read shows a Void-Switch main block + 45 Outemu keys on this
  board, so the two-color preview will be visible immediately.

## Key values on the board + no-overlap travel text (this request)

- REA direct static reads (`2a8cd9c4.js` template + `d9f04973.js`
  renderer helpers): the key-name slot of `KeyBox_KeyMappings_RT`
  (`#text_keyCode`, font-size 12, centered, tspan y=26.232) is filled
  from the `keyCodeDisplayText` prop — i.e. the key's CURRENT value
  (binding label), not the original cap name. Helper `ue()`: when the
  label contains spaces it re-tspans the words stacked vertically
  (line height font*1.2), each line centered at x=width/2, block
  vertically centered (`y=(h-lines*1.2*font)/2+font`); overflow is
  allowed in the original (svg `overflow="visible"`).
- Travel text anchors (template tspans, 42-unit key box): press
  `#FE4545` x=3, y=39.488, lift `#0066FF` x=22 (LEFT-anchored, so the
  same-px split never collides in the browser); `he()` shifts lift
  right by (keyW−42) → glued to the right edge on wide keys; `pe()`
  sets both travel fonts to `8*(4/(precision+2))` so added decimals
  keep the printed width ~constant. Our egui font advances are wider
  than the browser's at small sizes, so values previously overlapped
  on normal 40-unit keys at 45% scale.
- Implemented: key text = `binding.short_label()` when non-empty else
  original label (original stays in hover tooltip); multi-word labels
  stacked like `ue()`; font fitted per key (12k shrinking until the
  block fits the key's own rect above the travel row). Travel numbers
  use the template anchors `left+3k` / `right−20k` and disjoint
  per-key fitting zones (press [left+3k, lift-anchor), lift (anchor,
  right−2k]), each number shrinking only within its own span → no
  horizontal overlap on any key. Board painter is now centered in the
  window (`Layout::top_down(Center)` wrapping the allocate_painter).

## Bigger keys, banded rows, Nerd arrows back, activation glow (this request)

- Overlap report: the modified keybind text could still touch the
  travel numbers. The card rows are now STACKED from bounded band
  heights (live bottom-anchored, press band above it, orig/binding
  sharing the rest, 50/50 when bound; both shrunk proportionally when
  the pair exceeds its zone) — vertical overlap is impossible, not
  merely unlikely. Base fonts reduced one notch (orig/binding 9.2k,
  live 6.2k); the `pe()` travel font is untouched for fidelity.
- Board further enlarged (~58% → ~66% of screen width, min 420px,
  scale 0.75..3.4) AND each key grows into its inter-key gaps
  (uniform 1.7k inflate inside the asset's 46-unit pitch over the
  40-unit key box) — bigger keys, unchanged geometry fidelity.
  The shipped preview now targets ~50% of the screen width (`ui.rs`,
  matching README); the 45/58/66% figures record earlier requests.
- Nerd-Font arrow glyphs restored (`matrix::arrow_glyph`) after the
  vector-polygon experiment: the user prefers the real icons, and the
  regular binding row now ALWAYS draws beneath for bound arrow keys —
  the vector branch previously replaced the whole card, hiding the
  modified text.
- Live-input activation hook: when the current depth reaches the key's
  configured press-travel point (`mm >= travel_press`), the fill turns
  near-black (`#0D0D0D`, auto-contrast text goes white) and a layered
  red glow (`PRESS_COL` #FE4545 strokes, alpha 0.65→0.08, up to
  9.6k px) radiates around the key.

## Restored two-line key card + vector arrows + live mm/% (earlier request)

(Superseded: the vector-arrow experiment became Nerd-Font glyphs
again in the section above, per user preference.)

- User preference over the driver's single `keyCodeDisplayText` slot
  (`d9f04973.js Ke`): each key is a card — ORIGINAL cap name on top
  (muted), CURRENT binding below (full color, combos compacted
  `"LCtrl + Up"`→`"LCtrl+Up"` for one line), red/blue press/release
  travels, and a live-depth row at the bottom (`mm` + `%`).
- Arrows: the Nerd-Font glyphs (`matrix::arrow_glyph`, Font Awesome
  `\u{f060}`..`\u{f063}`) degrade to missing-text when the system icon
  font is absent; the four direction keys now draw vector polygons
  (`draw_arrow`: head triangle + stem, hid 79/80/81/82) so arrows
  always render.
- Live percent reference: `FULL_TRAVEL_MM = 4.0` (nominal magnetic
  total travel; no explicit %-of-total conversion exists in the driver
  JS — REA grep) — depth tint switched from the old 3.4 heuristic to
  the same 4.0 constant, legend states the reference.
- Board enlarged as requested: ~45% → ~58% of screen width
  (min 380px, scale clamp 0.7..3.4); per-key fitting zones keep every
  row inside its key at any size.

## Profiles are active-profile anchored + preview 45% (earlier request)

- REA `analyze_javascript_application` (scoped `7a5b12c9.js`+
  `index.5627ed60.js`) → corroborated by direct static reads:
  - `setCurrentProfile(t)` = `sendMsg([FEA_CMD_SET_PROFILE=4, t], Bit7)`
    + vendor sleep; `getCurrentProfile()` = `commonMsg([132], Bit7)[1]`.
  - Startup: `const t = await this.dev.getCurrentProfile() ?? 0;
    当前按键层 = t` — the UI selector always mirrors the device.
  - Layer-change watcher `___check当前按键层索引变化`: profile change →
    `await setCurrentProfile(t)` **then** re-read (`___获取所有按键信息`);
    Fn/son-layer change (`___checkFn层索引变化`) sends nothing extra —
    Fn reads/writes carry the profile byte (`[144/16, sys, profile, …]`).
- Live confirmation (read-only + cmd-4 switches, restored to profile 0):
  `[138, P, …]` returns the ACTIVE profile's base keymap for every P
  (all 4 profile bytes Identical md5), while `[144, sys, P, …]` +
  `[138]`+ travels DO honor P (Fn p0 vs p2 differ; travels p0 =
  0.50 mm uniform, p2 = 2.00/2.80 mm — per-profile data). cmd 4 verified
  via cmd 132 read-back (2 → 1 → 3 → 0). So the old UI (no cmd 4) always
  showed the profile the keyboard was on — that's why profile switching
  "did nothing".
- Implemented: `hid::set_profile`/`get_profile`; UI syncs the selector
  from cmd 132 on every Read-all; profile-dropdown change sends cmd 4
  before a full Read-all fetch (profile turns also span travels, which
  the earlier bindings-only refetch missed); `ensure_active_profile`
  guard before key/switch writes; CLI `profile read|set N`; per
  keymap-read help notes base-layer anchoring.
- Board preview now targets 45% of the screen width (responsive,
  `screen_rect * 0.45`), replacing the full-width layout.

## Fn connector/cluster quads named + preview card rework (this request)

- REA `analyze_javascript_application` (app tree, scoped `74d1dc6b.js` +
  `index.5627ed60.js` + `df09d59e.js` + `7a5b12c9.js`) →
  `ev_f6353762a596f30cb1828ceb285f54b5342bf855175e4d06704cfd45dd4638cb`
  (subject sha256 `d78e29ed825b…12079`; module `74d1dc6b.js` artifact
  sha256 `02328f27df…9d8d5` exports exactly
  `Ry5088_womier_sk75he_europe_3m_8k_8k`), corroborated by direct static
  reads (python/ripgrep) over the extracted tree.
- Naming-source sweep — the original driver does not name these quads
  either. The 176-entry master key table (`{value,scanCode,hidCode,…}`,
  hidCode = big-endian quad) contains only the tag-10/13 rows already in
  our table (+`[13,6,x]` ledMode1/2) — no tag-14 rows at all; `uw`/`fL`,
  `O_`/`Fa` (only `Ie.PAIR=[10,5,0,0]`, labeled "PAIR"), per-device
  default modules (bare arrays), the SK75 ui_info `7d1d89fb.js`
  (geometry + displayText only), the help device (`helpRy5088` →
  `fa78cffb.js` stub), all i18n dictionaries, and the firmware command
  surface (`FEA_CMD_GET_FN` is the plain Fn keymap read; no
  function-description getter): none carry connectivity/RGB Fn labels.
  The board overlay types them `ConfigUnknown` → `系统按键`
  ("System") — same as our previous build.
- Identification: the SK75 factory Fn default is the device module
  `74d1dc6b.js` (loader `df09d59e.js`:
  `ry5088_womier_sk75he_europe_3m_8k_8k` → `import("./74d1dc6b.js")`),
  128-quads `defaultFnMatrix`/`defaultFnMacMatrix` matching the live
  `keymap read --fn win` slot-for-slot (slot 8 Q `[14,0,0,0]`, 14 W
  `[14,0,1,0]`, 20 E `[14,0,2,0]`, 26 R `[14,0,5,0]`, 32 T `[14,0,6,0]`,
  52 M `[10,5,3,0]`, 77 Left `[13,5,0,2]`, 79 Bksp `[10,8,0,0]`,
  85 End `[13,5,1,0]`, 86 PgUp `[13,3,2,0]`, 87 PgDn `[13,3,1,0]`, 89
  Right `[13,5,0,1]`, 0 Esc `[10,2,0,0]`, 17 LGui `[10,3,0,0]`).
  Family statistics across ~100 sibling modules: `[14,0,x]` exists only
  on three-mode boards (five quads on Q/W/E/R/T); `[13,5,a,b]` /
  `[13,3,±]` sit on the arrow cluster mirroring the labelled
  `[13,2,±]` brightness pair; `[10,2]/[10,3]/[10,8]` appear on every
  board including wired-only ones.
- Naming (operator-verified panel answers): Fn+Q/W/E/R/T = Bluetooth
  1/2/3, 2.4 GHz, Wired; Fn+Left/Right = LED Direction-/LED Direction+;
  Fn+Win = Win-key lock (NOT a Win/Mac switch); Fn+Backspace = battery
  level; Fn+M = Windows/Mac mode switch. Operator follow-up: Fn+End
  (`[13,5,1,0]`) changes the LED color → named `LED Color`; Fn+Esc =
  `Factory Reset` confirmed. Still inferred from families, flagged in
  the table comment: Fn+PgUp/PgDn = LED Speed+/LED Speed- (`[13,3,±]`).
- Implemented: `NATIVE_FUNCTIONS` 51 → 65 entries, all byte-exact
  (round-trip `quad()` asserted); decode now names the 14 System slots
  of Fn win/mac — live `keymap read --fn win` shows Bluetooth 1 on Q,
  2.4 GHz on R, Wired on T, Win Lock on LGui, Win/Mac Switch on M,
  Battery Level on Backspace, LED Direction-/+ on Left/Right,
  LED Color on End, LED Speed+/- on PgUp/PgDn, Factory Reset
  on Esc. `function_options()` picks them up automatically (15 original
  media entries + 65 natives in the rebind picker). Tests:
  `sk75_fn_cluster_quads_are_named_functions` (labels + quad
  round-trips; `[10,5,0,0]` stays Unknown/System) and extended picker
  assertions; 32/32 pass.
- Preview card rework per request: the demo's switch-number corner
  badge (`#g_switchPath_and_switchNumberText`: notch + 1-based switch
  number) is removed — colors still map to switch types via the legend
  (legend hint reworded). Row bands reordered: red/blue
  press/release travels return to the very bottom (the driver template
  position, press `left+3k`/lift `right-20k` anchors unchanged), the
  live mm+% depth row moves into its own band directly above them, and
  a spacer band (`1.3k`, fitted like the text rows) now separates the
  original cap row from the current binding row.

## Correct op-252 switch write + key Reset defaults (2026-10-22)

- REA `analyze_javascript_application` (scoped `7a5b12c9.js` +
  `index.5627ed60.js` + `74d1dc6b.js` + `7de29f04.js` + `f01f4a92.js`)
  → parent
  `ev_f2e38f5e27a1a845a92bee0cd06911038643323135906d53c482aa060074a1c4`,
  summary
  `ev_270e853addc8bff0bf3bc843a9d0136049659aaddc2951c2f141d5c100ceed5b`.
- Switch write root cause: JS `_sendMagnetismInfoSimpleCMD(t,e,n,i)` builds
  an 8-byte header (`s[0]=101,s[1]=op,s[2]=0,s[3]=slot,s[4]=apply?1:0`,
  rest zero), then sends `[...s, ...i]`; `___encodeCmd` writes the Bit7
  checksum at index 7 over 0..6. The axis code therefore ships at **byte
  8**. Our old request `[101,252,0,slot,1,code]` was padded to 9 bytes by
  `encode_cmd`, so the firmware read 0 at byte 8 and stored Gaote
  (code 0) on every write. The corrected request is
  `[101,252,0,slot,1,0,0,ck,code]` (`src/switches.rs`,
  `write_switch_type`).
- Live proof (id 3804): writing the corrected packet stored code 13 on
  normal slot 41 and on system slot 71 (base default `[10,1,0,0]`, the Fn
  key); both read back 13. Earlier broken writes had turned slots
  0/8/14/33 into 0; all touched slots were restored to the installed
  Void Switch code 69 and re-read as 69. Direct matrix-slot targeting
  therefore persists for system keys; the original driver's
  `findIndexInDefaultMatrix(original,index)` lookup can miss keys whose
  default quad is not a plain HID.
- Reset (原驱动 恢复默认): Base single-key reset
  (`___单按键恢复按键`) writes `[0,0,hid,0]` for the key's original HID;
  Fn reset (`___恢复fn按键`) calls `setFnKeyConfigSimple` with
  `original===skey===key===key2`, and that method special-cases such
  combos to `defaultFnMacMatrix`/`defaultFnMatrix.slice(slot*4, +4)`.
  Our UI Reset restores the exact factory quad for the current layer:
  Base from the device `defaultMatrix` (`MATRIX_2518`/`MATRIX_3804`,
  system quads included), Fn from `defaultFnMatrix`/
  `defaultFnMacMatrix`. The 2518 module has no mac table; the shared base
  class (`7a5b12c9.js`) sets `defaultFnMacMatrix = this.defaultFnMatrix`,
  and both 2518/3804 win tables are byte-identical.
- Fn defaults added to `src/matrix.rs`: `FN_MATRIX_3804_WIN`,
  `FN_MATRIX_3804_MAC` (differ at slots 17/18/42/48/54),
  `FN_MATRIX_2518_WIN`/`_MAC` aliases, plus `default_matrix` /
  `default_fn_matrix` selectors. `keymap::default_quad` fetches one
  slot's Base/Fn default. The picker's old Clear button is gone; Reset
  sits next to Write key and is available for every binding type.
- CLI: `keymap write` now accepts `--fn win|mac` (ops 16 vs 10) so the
  Fn path is testable. Live check: Fn-win slot 8 `[14,0,0,0]`
  (Bluetooth 1) → temp `[3,0,234,0]` read back Volume- → restored
  `[14,0,0,0]` read back Bluetooth 1. Base slot 15 had been left
  non-default by earlier tests; writing its default `[0,0,22,0]` cleared
  the changed marker (S).
- Tests: `switches::single_write_layout_matches_driver` (code at byte 8,
  checksum at 7), `matrix::fn_defaults_match_driver_tables`,
  `matrix::default_matrix_picks_by_device_id`,
  `keymap::reset_defaults_cover_base_and_fn_layers`; `cargo test`
  35/35 pass in both binaries.

## UI-only revision + per-key travel calibration (2026-10-22)

REA parent `ev_01e3780aaaa6fa4af10c93b57e89c1e7b2e517ab83d646da07edc5026c1d2e35`
(`class C` in `7a5b12c9.js`, `_getJiaoZhunXinXi`,
`_getJiaoZhunXinXiLoop`, `setJiaoZhunKaiGuan`) plus the calibration UI
component `mj`/guide modal in `index.5627ed60.js`.

### Command sequence

The original `setJiaoZhunKaiGuan(true)` path is preserved without
blocking the GUI thread:

1. `SET_MAGNETISM_CAL (28)` with payload `1` (min-travel calibration on).
2. Wait 2 s, then `28` with payload `0` (min-cal off).
3. Immediately `SET_MAGNETISM_MAXIMUM_CALIBRATION (30)` with payload `1`
   (max-cal on).
4. The UI waits an additional 500 ms before enabling the guide's Start
   button (original state becomes `ing_调用中`).
5. Poll live travel with `GET_MULTI_MAGNETISM (229)`, op `254`, four
   pages (`_getMulitMagnetismCMD(254,4,1,1)`), decode 128 little-endian
   u16 values.
6. Finish sends `30` with payload `0`. Cancel, read failure, and window
   close also send `28`/`30` off, so the keyboard is never left with
   min/max calibration armed.

### Threshold and overlay

- Threshold source: `version.rf ?? version.usb`. `getRFVersion` is cmd
  `128` (`r[2]<<8|r[1]`, zero means absent); `getUSBVersion` is cmd
  `143` (`r[8]<<8|r[7]`).
- Original expression: `const y = version.rf ?? version.usb; const f =
  y && y < 768 ? 1 : 300;`. Modern firmware therefore needs raw `300`
  (3.00 mm at scale 100); versions below 768 use `1`.
- Overlay `#74EBD5`: `0 < raw < threshold` → bottom-up height
  `raw/threshold` at opacity 0.3; `raw >= threshold` → 100% height at
  opacity 0.8 and the key is added to `calibratedKeyCodeSet`.
- Calibrated keys remain at 100% / 0.8 after release. Releasing does
  not remove them. The original "Finish" button is not gated, but this
  UI intentionally gates it on every expected key because the requested
  flow requires pressing all keys.
- The Fn key is included. Both matrices encode it as the extended marker
  `[10,1,0,0]` (`167837696`), matching `matrixToHid` and the layout's
  `Fn` rect. Variant 3804 maps all 81 layout keys; variant 2518 maps 80
  because its matrix has RCtrl (`228`) where the shared UI geometry has
  AltRight (`230`).

### UI placement

The calibration entry is the **Travel Calibration** card to the left of
the keyboard preview in the Keyboard tab. Clicking it opens the English
guide modal and starts the non-blocking startup sequence; Start is
disabled until step 4 completes. After Start the card shows `done/total`,
a progress bar, **Finish calibration** (disabled until all expected keys
lock), and **Cancel calibration**. The normal live-depth tint/glow is
suppressed during calibration so the cyan overlay is unambiguous.

Tests: `proto::calibration_threshold_matches_driver`,
`layout::fn_maps_to_extended_marker_slot`,
`layout::every_layout_key_has_a_unique_slot`, and the `ui::tests`
locking/threshold/all-key/gating/overlay tests.

## Per-key activation travel + calibration completion (2026-10-22)

REA `analyze_javascript_application` scoped to `index.5627ed60.js`,
`7a5b12c9.js`, and `60ee4367.js` produced parent
`ev_08733d6da723c1075eb5a3f4c55af41a5b60cc0af2e7e374197035e29a200ae0`
and summary
`ev_b75b2da590e82a2a1fa4a4b99b217dad69b8115bab2a66b26453485a1ca42ae2`
(642,249 AST nodes, 1,927 findings, partial due to the result budget).
The travel panel evidence is `createState_行程设置`, `设置按下`,
`设置抬起`, and the `O` clamp in `index.5627ed60.js`; the protocol
evidence is `_sendMagnetismInfoSimpleCMD`, `setMagnetismInfoSimple`, and
`getMultiMagnetismInfo` in `7a5b12c9.js`. Precision scaling is the
existing `60ee4367.js` path.

### Defaults and limits

- Minimum press/release travel: `0.1 mm`.
- Default press travel: `2.0 mm`; default release travel: `2.8 mm`.
- Default deadzone: `0.3 mm` when op `6` has no usable value. The
  original `get死区()` clamps the selected key's deadzone to `0..=1 mm`
  before using it for the travel cap. The deadzone editor itself uses the
  original `设置死区`/`get死区最大值` limits: `0..=1 mm` (`0..=4 mm` below
  firmware 768), step `1 / scale` mm.
- Full travel is selected exactly like the original:
  `const y = version.rf ?? version.usb; y && y < 768 ? 4.0 : 3.3`.
  RF therefore takes precedence over USB. Zero/absent counts as modern
  firmware (`3.3 mm`). This build's `Jn.轴体行程最大值` is the constant
  function `f => 3.3`, so all SK75 switch types share the 3.3 mm
  configurable maximum; only firmware below 768 returns 4.0.
- Both slider maxima are `full_travel - deadzone` (the original `O`
  bound); the release setter additionally clamps release to that same
  maximum.
- Slider steps are one raw unit, i.e. `1 / scale` mm. The UI commits the
  slider's raw value directly in u16 LE, so the displayed step and the
  written step always match. The original converts its mm double with
  `const l = value * get磁轴行程步进倍数(); c = [l & 255, l >> 8]`;
  the bitwise ops truncate toward zero (no rounding). `proto::mm_to_raw`
  reproduces that for the mm constants/defaults (including the original's
  `0.29 * 100 -> 28` truncation), normalising the f32 input to six
  decimals first so 2.8/3.3 keep their exact integer raw values.
- The UI loads the selected key's current op-0/op-1 values, falls back
  to 2.0/2.8 for zero/unread values, and clamps both to the computed
  limits.

### Single-slot write

The original `_sendMagnetismInfoSimpleCMD(op, slot, last, data)` builds
an 8-byte header and appends the payload:

```text
[101, op, 0, slot, last, 0, 0, checksum] + [raw_lo, raw_hi]
```

- `op 0` is press `TRAVEL`; `op 1` is release `LIFT_TRAVEL`; `op 6`
  writes and reads `DEADZONE`.
- `setMagnetismInfoSimple(newState, previousState)` only pushes an op when
  the new value differs from the key model loaded by the previous read:
  `"travel" in t && ("travel" in e && t.travel !== e.travel || !("travel" in e))`,
  and likewise for `liftTravel` and `deadZoneTravel`. Missing previous
  fields count as changed. **Write key** therefore diffs the pending
  values against the last op-0/op-1/op-6 read and sends only the changed
  ops in order (`0, 1, 6`); if nothing changed it sends no packet and no
  vendor sleep.
- `last = 1` applies the queued change and is set only on the final
  changed op in the batch. One 100 ms vendor sleep follows a non-empty
  travel/deadzone batch.
- The op-252 switch write is also conditional in the original
  (`支持更换轴体 && "轴体" in t && e?.轴体 !== t.轴体`); it is sent after
  the travel batch with `last = 1`, followed by another 100 ms sleep.
  When the switch code is unchanged, no op-252 packet is sent.
- The toggle-off path sends the Press value to both op 0 and op 1; the
  toggle-on path sends the pending press and release values separately.
  This matches the original `设置按下`/`设置抬起` model while keeping
  edits pending until **Write key**. The diff is applied afterwards, so
  e.g. toggle-off with an unchanged Press but a stored release of 2.8 mm
  sends only op 1.
- Op `0`, op `1`, and op `6` reads each use four pages
  (`GET_MULTI_MAGNETISM` op, pages 0–3) = 256 bytes = 128 u16 LE
  values. Press/release are refreshed together with switch types, and
  deadzone is refreshed at the same time.

### Editor behavior

- **Press travel** is always visible. Moving any slider only updates the
  pending UI value; no HID write or read happens until **Write key**.
- With **Set press and release travel separately** off, **Write key**
  writes the same Press value to both ops and clamps it to full travel
  minus deadzone. Enabling the checkbox exposes **Release travel**; both
  pending values then commit to their own ops.
- The **Deadzone** slider edits op 6. Its maximum is 1.0 mm (4.0 mm
  below firmware 768); changing it also clamps the pending press/release
  values. The travel cap itself keeps the original 0–1.0 mm
  `get死区()` clamp.
- After a successful **Write key**, the UI waits an extra 200 ms, then
  performs the full Read all (keyboard info, bindings, switch types,
  travels and deadzone), reselects the same key, re-seeds every picker,
  and reports whether the read-back matched the requested values.
- The separate-release checkbox is UI-only state, matching the original
  `is单独设置按下抬起行程_非RT` toggle; it is not stored on the keyboard.
  On every read/selection sync it is derived solely from the stored values:
  on when `press != release`, off when they are equal or unknown. It stays
  a normal control while the user is editing, but no per-slot choice is
  remembered after a read.
- The keyboard persists the actual travel/deadzone values; there is no
  per-key mode flag to read back.

### Post-write read settling (hardware-verified)

The keyboard applies travel writes asynchronously. Probing the attached
SK75 TMR (`3151:5030`) showed that a `GET_FEATURE` issued immediately after
a write is answered with the *request payload itself* for roughly
150–180 ms, then normal data resumes. The original driver avoids this
window by updating its local model after a write instead of re-reading.
This recreation performs a full Read all, so it now:

1. keeps the original 100 ms vendor sleep after the final write;
2. waits an additional 200 ms before starting the refresh; and
3. detects a request echo in data-bearing reads (`GET_MULTI_MAGNETISM` and
   keymap pages) and retries up to four times at 50 ms intervals. If the
   echo persists, the read fails with an error instead of displaying the
   echoed bytes as keymap/travel values.

Hardware verification on the attached keyboard, using the same Rust
transport and packet builders (`slot 72`, all values restored afterwards):

```text
original op0/op1/op6 slot 72: (50, 50, 5)
after press-only write (60, 50, 5): (60, 50, 5)
after release-only write (60, 60, 5): (60, 60, 5)
after deadzone-only write (60, 60, 10): (60, 60, 10)
restored: (50, 50, 5)
```

This confirms op `0` changes press travel, op `1` changes release travel,
op `6` changes deadzone, and each write leaves the other fields untouched.
Pacing is now the original 10 ms before send and 10 ms before read for
normal traffic; the live op-254/255 path retains its explicit 1 ms pair.

### Calibration completion fix

Previously the cyan calibration overlay and locked bars remained after
**Finish calibration**, hiding normal switch colors and live input. The
overlay is now `Active`-phase only. Finish/cancel/failure:

1. sends cmd 30 off (and cmd 28 off where needed);
2. clears calibration raw/calibrated state and preview bars;
3. restores the pre-calibration live-polling state;
4. takes a fresh preview snapshot so the normal board is visible
   immediately.

The max-calibration-off command can make the hidraw node re-enumerate.
When live input was enabled before calibration, polling therefore retries
for 5 s (every 120 ms) instead of disabling itself on the first open
failure. The live-input checkbox is disabled while calibration is
running to prevent conflicting state changes. Cancel, error, and window
close use the same restoration path.

### Verification

`cargo fmt --all`, `cargo test --all-targets` (58/58), and
`cargo build --bin sk75-tmr-ui` pass. Travel tests cover defaults/limits,
toggle-off same-value commits, toggle-on separate commits, diff-only op
selection/order (`0,1,6`), final-op-only `last=1`, unknown-previous
handling, JS truncation scaling, deadzone firmware maxima and op-6 write
layout, request-echo detection, release-mode derivation from stored
values, Active-only overlay behavior, calibration running state, and
polling restoration.
