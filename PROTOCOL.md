# SK75 TMR protocol notes (REA-derived)

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
  is the separate firmware updater, not the driver.

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
  display POM mapping; single-key write `[101,252,0,slot,1,code]`,
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
