//! `sk75-tmr-ui` — Linux desktop recreation of the WOMIER driver UI.
//!
//! Layout mirrors the original Electron panel (`dist/index.html` title
//! `GearHub`, `dist/js/f70eb088.js` device card with firmware-version +
//! firmware-upgrade and IOT-version + check-update rows): a device card with a
//! **read keyboard information** action, plus TMR analog + calibration
//! tabs backed by the same `hid`/`proto`/`matrix`/`device_info` code as
//! the CLI. REA: `analyze_javascript_application` summary `ev_a9d5046b…`
//! + module page `ev_43d23e55…`.

mod device_info;
mod hid;
mod keymap;
mod layout;
mod matrix;
mod proto;
mod switches;

use device_info::{read_keyboard_info, KeyboardInfo};
use eframe::egui;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Device,
    Keyboard,
    Calibration,
}

struct KeyBar {
    slot: usize,
    hid: u32,
    name: String,
    raw: u16,
    mm: f32,
}

struct App {
    vid: u16,
    pid: u16,
    path: String,
    info: KeyboardInfo,
    tab: Tab,
    status: String,
    bars: Vec<KeyBar>,
    polling: bool,
    polls: u64,
    poll_window_start: Option<Instant>,
    hz: f64,
    last_layer: Option<(u8, u8)>,
    cal_busy: bool,
    cal_msg: String,
    // Remap tab state (mirrors driver key-rebind panel: profile/Fn layers,
    // per-key binding list, binding-type options, single-key write).
    remap_profile: u8,
    remap_fn: u8, // 0=base, 1=win, 2=mac
    remap_entries: Vec<keymap::KeyEntry>,
    remap_selected: Option<usize>,
    remap_bind_type: String,
    remap_combo: [u32; 3],
    remap_mouse: usize,
    remap_function: usize,
    remap_gamepad: usize,
    remap_macro_type: String,
    remap_macro_idx: u8,
    remap_msg: String,
    switch_codes: Vec<u8>,
    // Configured activation travel (ops 0/1, JS 触发行程/抬起行程) for the
    // keyboard-preview per-key values; `travel_digits` matches the driver's
    // `travelPrecision` getter (decimals of the press-travel step, default 2).
    travel_press: Vec<u16>,
    travel_lift: Vec<u16>,
    travel_digits: usize,
    selected_switch: usize,
    switch_msg: String,
}

impl App {
    fn new() -> Self {
        let info = KeyboardInfo::offline("SK75 TMR", proto::VID_RONGYUAN, proto::PID_SK75_TMR);
        Self {
            vid: proto::VID_RONGYUAN,
            pid: proto::PID_SK75_TMR,
            path: String::new(),
            info,
            tab: Tab::Device,
            status: String::from("Press “Read keyboard information”."),
            bars: Vec::new(),
            polling: false,
            polls: 0,
            poll_window_start: None,
            hz: 0.0,
            last_layer: None,
            cal_busy: false,
            cal_msg: String::new(),
            remap_profile: 0,
            remap_fn: 0,
            remap_entries: Vec::new(),
            remap_selected: None,
            remap_bind_type: String::from("Key"),
            remap_combo: [0, 30, 0],
            remap_mouse: 0,
            remap_function: 5,
            remap_gamepad: 0,
            remap_macro_type: String::from("repeat_times"),
            remap_macro_idx: 0,
            remap_msg: String::new(),
            switch_codes: vec![0u8; 128],
            travel_press: Vec::new(),
            travel_lift: Vec::new(),
            travel_digits: 2,
            selected_switch: 0,
            switch_msg: String::new(),
        }
    }

    fn path_opt(&self) -> Option<&str> {
        if self.path.trim().is_empty() {
            None
        } else {
            Some(self.path.trim())
        }
    }

    fn read_info(&mut self) {
        // Driver startup: sync the UI layer selector to the keyboard's
        // actual active profile (`___获取初始按键信息`:
        // `const t = await this.dev.getCurrentProfile() ?? 0;
        //  当前按键层 = t`), so the UI shows the profile the device is on.
        match hid::open_sk75(self.vid, self.pid, self.path_opt(), None) {
            Ok(dev) => match dev.get_profile() {
                Ok(Some(p)) if p <= 3 => self.remap_profile = p,
                Ok(Some(p)) => self.status = format!("Device reports invalid profile {p}; keeping selector."),
                Ok(None) => {}
                Err(e) => self.status = format!("Current profile read failed: {e:#}"),
            },
            Err(_) => {}
        }
        self.info = read_keyboard_info(self.vid, self.pid, self.path_opt(), None);
        self.status = match &self.info.error {
            Some(e) => format!("Read finished with note: {e}"),
            None => String::from("Keyboard information read OK."),
        };
        // Auto-fetch Keyboard tab data (bindings + switch types) whenever
        // device info is read, so the Keyboard menu is already populated.
        self.read_remap();
        self.read_switches();
        self.last_layer = Some((self.remap_profile, self.remap_fn));
        // One-shot analog snapshot so the on-screen board preview shows
        // current depths immediately, even before the live overlay runs.
        self.snapshot_preview();
    }

    /// Best-effort single op254 snapshot populating `bars` without touching
    /// the continuous `polling` flag / Hz counters.
    fn snapshot_preview(&mut self) {
        let dev = match hid::open_sk75(self.vid, self.pid, self.path_opt(), None) {
            Ok(d) => d,
            Err(_) => return,
        };
        let scale = dev.travel_scale().unwrap_or(self.info.scale.max(1));
        let travel = match dev.live_travel_raw() {
            Ok(v) => v,
            Err(_) => return,
        };
        let mat = matrix::default_matrix(self.info.device_id);
        let mut bars = Vec::new();
        for (slot, h) in matrix::populated_keys(mat) {
            let r = travel.get(slot).copied().unwrap_or(0);
            if r == 0 {
                continue;
            }
            bars.push(KeyBar {
                slot,
                hid: h,
                name: matrix::hid_name(h),
                raw: r,
                mm: proto::raw_to_mm(r, scale),
            });
        }
        bars.sort_by_key(|b| std::cmp::Reverse(b.raw));
        self.bars = bars;
    }

    fn poll_once(&mut self) {
        let dev = match hid::open_sk75(self.vid, self.pid, self.path_opt(), None) {
            Ok(d) => d,
            Err(e) => {
                self.status = format!("{e:#}. Hint: wired mode + udev/99-sk75-tmr.rules");
                self.polling = false;
                return;
            }
        };
        let scale = dev.travel_scale().unwrap_or(self.info.scale.max(1));
        self.info.scale = scale;
        // Live loop mirrors getJiaoZhunXinXiLoop: 1 ms HID pacing with
        // no inter-poll sleep, so the rate is HID round-trip bound (~83 Hz).
        let travel = match dev.live_travel_raw() {
            Ok(v) => v,
            Err(e) => {
                self.status = format!("Analog read failed: {e:#}");
                self.polling = false;
                return;
            }
        };
        let mat = matrix::default_matrix(self.info.device_id);
        let mut bars = Vec::new();
        for (slot, h) in matrix::populated_keys(mat) {
            let r = travel.get(slot).copied().unwrap_or(0);
            if r == 0 {
                continue;
            }
            bars.push(KeyBar {
                slot,
                hid: h,
                name: matrix::hid_name(h),
                raw: r,
                mm: proto::raw_to_mm(r, scale),
            });
        }
        bars.sort_by_key(|b| std::cmp::Reverse(b.raw));
        self.bars = bars;
        self.polls += 1;
        let now = Instant::now();
        match self.poll_window_start {
            Some(t0) if now.duration_since(t0).as_secs_f64() >= 1.0 => {
                self.hz = self.polls as f64 / now.duration_since(t0).as_secs_f64();
                self.polls = 0;
                self.poll_window_start = Some(now);
            }
            None => self.poll_window_start = Some(now),
            _ => {}
        }
        self.status = format!(
            "Polled {} pressed keys (op 254, scale {scale}/mm, 1ms pacing{})",
            self.bars.len(),
            if self.hz > 0.0 {
                format!(", {:.0} Hz", self.hz)
            } else {
                String::new()
            },
        );
    }

    fn read_remap(&mut self) {
        let dev = match hid::open_sk75(self.vid, self.pid, self.path_opt(), None) {
            Ok(d) => d,
            Err(e) => {
                self.remap_msg = format!("{e:#}. Hint: wired mode + udev/99-sk75-tmr.rules");
                return;
            }
        };
        let mat = matrix::default_matrix(self.info.device_id);
        let blob = if self.remap_fn == 0 {
            match dev.read_keymap(self.remap_profile) {
                Ok(b) => b,
                Err(e) => {
                    self.remap_msg = format!("Keymap read failed: {e:#}");
                    return;
                }
            }
        } else {
            let sys = if self.remap_fn == 2 {
                keymap::FnSys::Mac
            } else {
                keymap::FnSys::Win
            };
            match dev.read_fn_keymap(self.remap_profile, sys) {
                Ok(b) => b,
                Err(e) => {
                    self.remap_msg = format!("Fn keymap read failed: {e:#}");
                    return;
                }
            }
        };
        self.remap_entries = keymap::decode_keymap(&blob, mat);
        self.remap_selected = None;
        let changed = self.remap_entries.iter().filter(|e| e.changed).count();
        self.remap_msg = format!(
            "Read {} keys ({} remapped) on {}.",
            self.remap_entries.len(),
            changed,
            if self.remap_fn == 0 {
                format!("profile {}", self.remap_profile + 1)
            } else {
                format!(
                    "Fn {} profile {}",
                    if self.remap_fn == 2 { "mac" } else { "win" },
                    self.remap_profile + 1
                )
            }
        );
    }

    /// Driver `___check当前按键层索引变化` never writes while the selector
    /// points elsewhere: profile switches send cmd 4 first (`setCurrentProfile`)
    /// because base-keymap reads (and, conservatively, writes) are anchored
    /// to the keyboard's ACTIVE profile — live-verified: `[138, P, …]`
    /// returns the active profile's map for every P (Fn cmd-144 reads do
    /// honor the profile byte).
    fn ensure_active_profile(&mut self) {
        let Ok(dev) = hid::open_sk75(self.vid, self.pid, self.path_opt(), None) else {
            return;
        };
        if let Ok(Some(p)) = dev.get_profile() {
            if p <= 3 && p != self.remap_profile {
                if let Err(e) = dev.set_profile(self.remap_profile) {
                    self.remap_msg = format!("Profile switch failed: {e:#}");
                }
            }
        }
    }

    fn apply_remap(&mut self) {
        let slot = match self.remap_selected {
            Some(i) => self.remap_entries.get(i).map(|e| e.slot),
            None => None,
        };
        let slot = match slot {
            Some(s) => s,
            None => {
                self.remap_msg = String::from("Select a key first.");
                return;
            }
        };
        let binding = match self.remap_bind_type.as_str() {
            "Disabled" => keymap::Binding::Disabled,
            "Mouse" => {
                let opts = keymap::mouse_options();
                let (_, q) = &opts[self.remap_mouse.min(opts.len() - 1)];
                keymap::Binding::Mouse {
                    label: q.to_vec().len().to_string(),
                    quad: *q,
                }
            }
            "Function" => {
                let opts = keymap::function_options();
                let (l, q) = &opts[self.remap_function.min(opts.len() - 1)];
                keymap::Binding::Function {
                    label: l.clone(),
                    quad: *q,
                }
            }
            "Gamepad" => {
                let opts = keymap::gamepad_options();
                let (l, q) = &opts[self.remap_gamepad.min(opts.len() - 1)];
                keymap::Binding::Gamepad {
                    label: l.clone(),
                    quad: *q,
                }
            }
            "Macro" => keymap::Binding::Macro {
                macro_type: self.remap_macro_type.clone(),
                index: self.remap_macro_idx,
            },
            _ => keymap::Binding::Combo {
                skey: 0,
                key: 0,
                key2: 0,
            },
        };
        // Rebuild the combo from the picked triple (skey/key/key2 slots).
        // Untouched entries round-trip byte-identical; new picks fill key,
        // then key2, then skey, so lone keys keep firmware default bytes
        // `[0,0,hid,0]`. Empty refuses, like the original
        // (`if(list.length===0)return`).
        let binding = match &binding {
            keymap::Binding::Combo { .. } => {
                let c = self.remap_combo;
                if c == [0, 0, 0] {
                    self.remap_msg = String::from("Pick at least one key for the combo.");
                    return;
                }
                let (skey, key2) = match (u8::try_from(c[0]), u8::try_from(c[2])) {
                    (Ok(s), Ok(k2)) => (s, k2),
                    _ => {
                        self.remap_msg =
                            String::from("Combo key out of one-byte range; re-pick.");
                        return;
                    }
                };
                keymap::Binding::Combo {
                    skey,
                    key: c[1],
                    key2,
                }
            }
            _ => binding,
        };
        // Rebuild proper labels for mouse (label is cosmetic; quad is what ships).
        let binding = match &binding {
            keymap::Binding::Mouse { quad, .. } => {
                let label = keymap::mouse_options()
                    .into_iter()
                    .find(|(_, b)| b == quad)
                    .map(|(l, _)| l)
                    .unwrap_or_default();
                keymap::Binding::Mouse { label, quad: *quad }
            }
            _ => binding,
        };
        self.ensure_active_profile();
        let dev = match hid::open_sk75(self.vid, self.pid, self.path_opt(), None) {
            Ok(d) => d,
            Err(e) => {
                self.remap_msg = format!("{e:#}");
                return;
            }
        };
        let res = if self.remap_fn == 0 {
            dev.write_key_simple(self.remap_profile, slot, binding.quad())
        } else {
            let sys = if self.remap_fn == 2 {
                keymap::FnSys::Mac
            } else {
                keymap::FnSys::Win
            };
            dev.write_fn_key_simple(self.remap_profile, sys, slot, binding.quad())
        };
        match res {
            Ok(()) => {
                let q = binding.quad();
                let written = format!(
                    "Slot {slot} -> {} [{},{},{},{}]",
                    binding.short_label(),
                    q[0],
                    q[1],
                    q[2],
                    q[3]
                );
                // Fetch Read-all state automatically; keep the key selected.
                self.refresh_keep_selected(slot, &written);
            }
            Err(e) => self.remap_msg = format!("Write failed: {e:#}"),
        }
    }

    /// Restore the selected key's factory binding, mirroring the original
    /// driver's 恢复默认: base layer from `defaultMatrix`, Fn layer from
    /// `defaultFnMatrix`/`defaultFnMacMatrix` for the active OS. Works for
    /// system keys too (e.g. base slot 71 -> `[10,1,0,0]`, the Fn key).
    fn reset_selected_key(&mut self) {
        let slot = match self.remap_selected {
            Some(i) => match self.remap_entries.get(i) {
                Some(e) => e.slot,
                None => {
                    self.remap_msg = String::from("Select a key first.");
                    return;
                }
            },
            None => {
                self.remap_msg = String::from("Select a key first.");
                return;
            }
        };
        let sys = match self.remap_fn {
            0 => None,
            2 => Some(keymap::FnSys::Mac),
            _ => Some(keymap::FnSys::Win),
        };
        // Base-layer writes are anchored to the active profile; Fn writes
        // honor the profile byte directly.
        if sys.is_none() {
            self.ensure_active_profile();
        }
        let dev = match hid::open_sk75(self.vid, self.pid, self.path_opt(), None) {
            Ok(d) => d,
            Err(e) => {
                self.remap_msg = format!("{e:#}");
                return;
            }
        };
        // Prefer the id reported by the connected device over cached info,
        // so Reset always picks the matching default matrix.
        let device_id = dev.device_id().unwrap_or(None).or(self.info.device_id);
        let quad = match keymap::default_quad(device_id, slot, sys) {
            Some(q) => q,
            None => {
                self.remap_msg = format!("No default available for slot {slot}.");
                return;
            }
        };
        let res = match sys {
            None => dev.write_key_simple(self.remap_profile, slot, quad),
            Some(s) => dev.write_fn_key_simple(self.remap_profile, s, slot, quad),
        };
        match res {
            Ok(()) => {
                let label = keymap::decode_quad(quad).short_label();
                let written = format!(
                    "Slot {slot} reset to default {} [{},{},{},{}]",
                    label, quad[0], quad[1], quad[2], quad[3]
                );
                self.refresh_keep_selected(slot, &written);
                // Re-seed the picker from the restored binding.
                if let Some(i) = self.remap_selected {
                    select_entry(self, i);
                }
            }
            Err(e) => self.remap_msg = format!("Reset failed: {e:#}"),
        }
    }

    /// Re-read bindings + switches after a write while keeping `slot` selected.
    fn refresh_keep_selected(&mut self, slot: usize, written: &str) {
        self.read_remap();
        self.read_switches();
        self.remap_selected = self.remap_entries.iter().position(|e| e.slot == slot);
        self.remap_msg = format!("{written}. Verified by re-read.");
    }

    /// Driver `___check当前按键层索引变化`: the profile (当前按键层) selector
    /// sends `setCurrentProfile` (cmd 4) so the keyboard's *active* profile
    /// follows the UI before any per-profile read/write. Without this the
    /// device (and queries) keep returning the previous profile — which is
    /// why the profiles appeared to never change.
    fn sync_device_profile(&mut self) {
        match hid::open_sk75(self.vid, self.pid, self.path_opt(), None) {
            Ok(dev) => match dev.set_profile(self.remap_profile) {
                Ok(()) => {
                    self.status = format!(
                        "Keyboard active profile -> {} (cmd 4).",
                        self.remap_profile + 1
                    );
                }
                Err(e) => {
                    self.remap_msg = format!("Profile switch failed: {e:#}");
                }
            },
            Err(e) => self.remap_msg = format!("{e:#}. Hint: wired mode + udev/99-sk75-tmr.rules"),
        }
    }

    fn read_switches(&mut self) {
        match hid::open_sk75(self.vid, self.pid, self.path_opt(), None) {
            Ok(dev) => match dev.read_switch_types_raw() {
                Ok(codes) => {
                    // Preselect the selected key's switch in the dropdown.
                    if let Some(i) = self.remap_selected {
                        if let Some(e) = self.remap_entries.get(i) {
                            let code = codes.get(e.slot).copied().unwrap_or(0);
                            if let Some(pos) =
                                switches::SK75_SWITCHES.iter().position(|(c, _)| *c == code)
                            {
                                self.selected_switch = pos;
                            }
                        }
                    }
                    self.switch_codes = codes;
                    self.switch_msg = String::from("Switch types read (op 252).");
                }
                Err(e) => self.switch_msg = format!("Switch read failed: {e:#}"),
            },
            Err(e) => self.switch_msg = format!("{e:#}"),
        }
        self.read_travels();
    }

    /// Configured per-key activation travel (op 0 press / op 1 release),
    /// the values the driver preview prints on every key. Refreshed together
    /// with the switch types, mirroring `getMultiMagnetismInfo`.
    fn read_travels(&mut self) {
        let Ok(dev) = hid::open_sk75(self.vid, self.pid, self.path_opt(), None) else {
            return;
        };
        let scale = dev.travel_scale().unwrap_or(self.info.scale.max(1));
        self.info.scale = scale;
        self.travel_digits = proto::travel_digits(scale);
        match dev.press_lift_travel_raw() {
            Ok((p, l)) => {
                self.travel_press = p;
                self.travel_lift = l;
                self.switch_msg.push_str(" Activation travels read (ops 0/1).");
            }
            Err(e) => {
                self.switch_msg.push_str(&format!(" Travel read failed: {e:#}"))
            }
        }
    }

    fn apply_switch(&mut self) {
        let slot = match self.remap_selected {
            Some(i) => match self.remap_entries.get(i) {
                Some(e) => e.slot,
                None => {
                    self.switch_msg = String::from("Select a key first.");
                    return;
                }
            },
            None => {
                self.switch_msg = String::from("Select a key first.");
                return;
            }
        };
        let (code, label) = switches::SK75_SWITCHES[self.selected_switch.min(5)];
        self.ensure_active_profile();
        match hid::open_sk75(self.vid, self.pid, self.path_opt(), None) {
            Ok(dev) => match dev.write_switch_type(slot, code) {
                Ok(()) => {
                    drop(dev);
                    // Read back like Write key: the old packet wrote code 0
                    // and still reported success, so verify persistence.
                    self.read_switches();
                    let actual = self.switch_codes.get(slot).copied().unwrap_or(0);
                    if let Some(pos) =
                        switches::SK75_SWITCHES.iter().position(|(c, _)| *c == actual)
                    {
                        self.selected_switch = pos;
                    }
                    self.switch_msg = if actual == code {
                        format!("Slot {slot} switch -> {label} ({code}). Verified by re-read.")
                    } else {
                        format!(
                            "Slot {slot} write returned {code}, but re-read shows {} ({}).",
                            switches::switch_display(actual),
                            actual
                        )
                    };
                }
                Err(e) => self.switch_msg = format!("Switch write failed: {e:#}"),
            },
            Err(e) => self.switch_msg = format!("{e:#}"),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.polling {
            // No inter-poll sleep: poll every frame and repaint continuously
            // for the max HID-bound rate, like the original live loop.
            self.poll_once();
            ctx.request_repaint();
        }
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("SK75 TMR — Linux driver");
                ui.separator();
                for (tab, label) in [
                    (Tab::Device, "Device info"),
                    (Tab::Keyboard, "Keyboard"),
                    (Tab::Calibration, "Calibration"),
                ] {
                    ui.selectable_value(&mut self.tab, tab, label);
                }
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::Device => device_tab(ui, self),
            Tab::Keyboard => keyboard_tab(ui, self),
            Tab::Calibration => calibration_tab(ui, self),
        });
    }
}

fn device_tab(ui: &mut egui::Ui, app: &mut App) {
    ui.heading("Keyboard information");
    ui.label("Recreates the driver panel (firmware/IOT version card + device data). Reading queries the keyboard over HID feature reports.");
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("VID");
        ui.add(egui::DragValue::new(&mut app.vid).hexadecimal(4, false, true));
        ui.label("PID");
        ui.add(egui::DragValue::new(&mut app.pid).hexadecimal(4, false, true));
        ui.label("hidraw path (blank = auto)");
        ui.text_edit_singleline(&mut app.path);
    });
    if ui.button("Read keyboard information").clicked() {
        app.read_info();
    }
    ui.separator();
    let info = &app.info;
    egui::Grid::new("info-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        row(ui, "Display", &info.display_name);
        row(ui, "VID:PID (hex)", &info.vid_pid_hex());
        row(ui, "VID:PID (dec)", &info.vid_pid_dec());
        row(ui, "Path", &info.path);
        row(ui, "Product", &info.product);
        row(ui, "Manufacturer", &info.manufacturer);
        row(ui, "Device ID", &info.device_id_text());
        row(ui, "Variant", &info.variant);
        row(ui, "USB version", &info.usb_version_text());
        row(ui, "Precision", &info.precision_text());
        row(ui, "Report rate", &info.report_rate_text());
        row(ui, "Keys", &info.key_count.to_string());
    });
    ui.separator();
    ui.label(format!("Status: {}", app.status));
    if let Some(e) = &info.error {
        ui.colored_label(egui::Color32::YELLOW, format!("Note: {e}"));
    }
    ui.small("No device? Use wired mode, install udev/99-sk75-tmr.rules, replug.");
}

fn row(ui: &mut egui::Ui, k: &str, v: &str) {
    ui.label(k);
    ui.monospace(v);
    ui.end_row();
}

fn keyboard_tab(ui: &mut egui::Ui, app: &mut App) {
    ui.heading("On-screen keyboard");
    ui.label("Same 81-key arrangement as the original driver (positions from its keymap asset). Each key shows its label, current binding and live press depth. Click a key to select it for rebinding.");
    ui.horizontal(|ui| {
        ui.label("Profile");
        egui::ComboBox::from_id_salt("kb-profile")
            .selected_text(format!("{}", app.remap_profile + 1))
            .show_ui(ui, |ui| {
                for p in 0..4u8 {
                    ui.selectable_value(&mut app.remap_profile, p, format!("{}", p + 1));
                }
            });
        ui.label("Layer");
        egui::ComboBox::from_id_salt("kb-layer")
            .selected_text(match app.remap_fn {
                1 => "Fn win",
                2 => "Fn mac",
                _ => "Base",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.remap_fn, 0, "Base");
                ui.selectable_value(&mut app.remap_fn, 1, "Fn win");
                ui.selectable_value(&mut app.remap_fn, 2, "Fn mac");
            });
        if ui.button("Read all (info + bindings + switches)").clicked() {
            app.read_info();
        }
        ui.checkbox(&mut app.polling, "Live input overlay (max speed)");
    });
    // Re-read the keyboard configuration whenever profile/layer changes,
    // mirroring the original driver's layer switch (setCurrentProfile cmd 4).
    // Also lazy-fetch on first Keyboard visit if device info is already
    // known but bindings are still empty (e.g. info was read before this
    // auto-fetch existed), so the preview never stays blank.
    let cur_layer = (app.remap_profile, app.remap_fn);
    if app.last_layer != Some(cur_layer) {
        let (first, prev) = match app.last_layer {
            None => (true, None),
            Some(p) => (false, Some(p)),
        };
        app.last_layer = Some(cur_layer);
        if !first {
            // Profile (当前按键层) changes: switch the keyboard's active
            // profile first (cmd 4), then "Read all" (bindings + switches +
            // travels + analog snapshot), mirroring
            // `___check当前按键层索引变化` -> setCurrentProfile ->
            // ___获取所有按键信息. Fn (son-layer) changes skip cmd 4 — the
            // driver's `___checkFn层索引变化` only re-reads Fn configs.
            if prev.map(|p| p.0 != cur_layer.0).unwrap_or(false) {
                app.sync_device_profile();
            }
            app.read_info();
        } else if app.remap_entries.is_empty() && app.info.device_id.is_some() {
            app.read_remap();
            app.read_switches();
            app.snapshot_preview();
        }
    }
    ui.label(format!("Status: {}", app.status));
    if !app.switch_msg.is_empty() {
        ui.small(&app.switch_msg);
    }
    let mat = matrix::default_matrix(app.info.device_id);
    let live: std::collections::HashMap<usize, f32> =
        app.bars.iter().map(|b| (b.slot, b.mm)).collect();
    // Sized to ~50% of the screen width (responsive) and CENTERED in the
    // window; keys additionally grow into their inter-key gaps, and per-key
    // text is fitted to each key rect.
    let screen_w = ui.ctx().screen_rect().width();
    let target = (screen_w * 0.50).max(420.0);
    let k = ((target - 16.0) / layout::CANVAS_W).clamp(0.75, 3.4);
    let (ox, oy) = (8.0, 8.0);
    let (resp, painter) = ui
        .with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            ui.allocate_painter(
                egui::Vec2::new(layout::CANVAS_W * k + 16.0, layout::CANVAS_H * k + 16.0),
                egui::Sense::click(),
            )
        })
        .inner;
    // Colors straight from the driver key template `KeyBox_KeyMappings_RT`
    // (dist/js/2a8cd9c4.js): press value #FE4545 bottom-left, release value
    // #0066FF bottom-right, selected outline rgba(253.94,68.77,68.77).
    const PRESS_COL: egui::Color32 = egui::Color32::from_rgb(0xFE, 0x45, 0x45);
    const LIFT_COL: egui::Color32 = egui::Color32::from_rgb(0x00, 0x66, 0xFF);
    const SELECT_COL: egui::Color32 = egui::Color32::from_rgb(0xFD, 0x44, 0x44);
    // Reference full-stroke travel for the live % readout and depth tint
    // (magnetic switches: ~4.0 mm nominal total travel).
    const FULL_TRAVEL_MM: f32 = 4.0;
    // Travel display decimals: driver `travelPrecision` getter.
    let digits = app.travel_digits;
    let scale_mm = app.info.scale.max(1);
    let of_slot = |raw: &[u16], slot: Option<usize>| -> Option<f32> {
        let r = *raw.get(slot?)?;
        (r != 0).then(|| proto::raw_to_mm(r, scale_mm))
    };
    let mut hover_info: Option<String> = None;
    for (lid, hid, x, y, w, hgt, label) in layout::KEYS {
        // Keys grow slightly INTO the inter-key gaps (asset pitch 46 vs the
        // 40-wide unit keys) so each key reads bigger without changing the
        // layout geometry; the fitted text sizes keep every row separated.
        let inf = 1.7 * k;
        let rect = egui::Rect::from_min_max(
            resp.rect.min + egui::Vec2::new(ox + x * k - inf, oy + y * k - inf),
            resp.rect.min
                + egui::Vec2::new(ox + (x + w) * k + inf, oy + (y + hgt) * k + inf),
        );
        let slot = layout::slot_for(hid, mat);
        let mm = slot.and_then(|s| live.get(&s).copied()).unwrap_or(0.0);
        let depth = (mm / FULL_TRAVEL_MM).clamp(0.0, 1.0);
        let selected = slot
            .and_then(|s| {
                app.remap_entries
                    .iter()
                    .position(|e| e.slot == s)
                    .map(|i| Some(i) == app.remap_selected)
            })
            .unwrap_or(false);
        let binding = slot
            .and_then(|s| {
                app.remap_entries
                    .iter()
                    .find(|e| e.slot == s)
                    .map(|e| e.binding.short_label())
            })
            .unwrap_or_default();
        let sw_code = slot
            .and_then(|s| app.switch_codes.get(s).copied())
            .unwrap_or(0xFF);
        let [sr, sg, sb] = switches::switch_color(sw_code);
        // Live-input activation: once the current depth reaches the key's
        // configured press-travel activation point, the fill turns black
        // (auto-contrast text goes white) and a layered red glow radiates
        // around the key so the actuation moment is unmistakable.
        let act_mm = of_slot(&app.travel_press, slot);
        let activated = mm > 0.0005 && act_mm.is_some_and(|a| mm >= a);
        // Key fill = the selected switch's driver color; live press depth
        // lightens it (kept aesthetic on the dark panel).
        let lit = |c: u8| (c as f32 + (255.0 - c as f32) * depth * 0.55).round() as u8;
        let fill = if activated {
            egui::Color32::from_rgb(0x0D, 0x0D, 0x0D)
        } else {
            egui::Color32::from_rgb(lit(sr), lit(sg), lit(sb))
        };
        let round = egui::CornerRadius {
            nw: (5.0 * k) as u8,
            ne: (5.0 * k) as u8,
            sw: (5.0 * k) as u8,
            se: (5.0 * k) as u8,
        };
        painter.rect_filled(rect, round, fill);
        // Activation glow: layered outer strokes (driver activation red,
        // alpha decaying outward) around the active key.
        if activated {
            for (grow, alpha) in [
                (0.0_f32, 0.65_f32),
                (2.4, 0.45),
                (4.8, 0.28),
                (7.2, 0.16),
                (9.6, 0.08),
            ] {
                let r = (5.0 * k + grow * k) as u8;
                painter.rect_stroke(
                    rect.expand2(egui::Vec2::splat(grow * k)),
                    egui::CornerRadius {
                        nw: r,
                        ne: r,
                        sw: r,
                        se: r,
                    },
                    egui::Stroke::new(2.0 * k, PRESS_COL.gamma_multiply(alpha)),
                    egui::StrokeKind::Outside,
                );
            }
        }
        // (The driver's switch-number corner badge stays off per request:
        // the legend below already maps fill colors to switch types.)
        // Layered key card: ORIGINAL cap on top (muted), a spacer band, the
        // CURRENT binding under it, then the live depth row in its own band
        // and finally the red/blue press/release travels at the very bottom
        // (the driver template position). Rows are stacked from bounded
        // band heights, so rows can no longer collide; all base fonts stay
        // a step smaller per earlier request while the card itself grew.
        let text_col = text_on(fill);
        // Arrow keys render the Nerd-Font glyph again (user preference over
        // the vector polygons: the real icon matches the modified-key text,
        // `load_nerd_font` puts it on the font stack at startup) and the
        // binding always draws in its own row beneath.
        let orig: &str = matrix::arrow_glyph(hid).unwrap_or(label);
        let bind_txt = binding.replace(" + ", "+");
        // Live depth (op 254) as mm + percent of the full stroke.
        let live_pct = ((mm / FULL_TRAVEL_MM * 100.0).round() as i32).clamp(0, 100);
        let has_live = mm > 0.0005;
        let live_txt = if has_live {
            format!("{mm:.digits$}mm {live_pct}%")
        } else {
            String::new()
        };
        let fit = |txt: &str, size: f32| {
            painter
                .layout_no_wrap(txt.to_owned(), egui::FontId::proportional(size), text_col)
                .rect
                .width()
        };
        let fit_font = |txt: &str, base: f32, zone: f32| -> f32 {
            if txt.is_empty() || zone <= 2.0 {
                return base;
            }
            let w0 = fit(txt, base);
            if w0 <= zone || base <= 3.0 {
                base
            } else {
                (base * zone / w0).max(3.0)
            }
        };
        // Base fonts: one notch smaller everywhere (orig/binding 9.2k, live
        // 6.2k); the driver pe() rule stays untouched for the red/blue row.
        let fbase = 9.2 * k;
        let of0 = fit_font(orig, fbase, (rect.width() - 8.0 * k).max(2.0));
        let bf0 = fit_font(&bind_txt, fbase, (rect.width() - 7.0 * k).max(2.0));
        let travel_font0 = (8.0 * k * 4.0 / (digits.max(1) as f32 + 2.0)).max(3.0);
        let has_trav =
            of_slot(&app.travel_press, slot).is_some() || of_slot(&app.travel_lift, slot).is_some();
        let lf0 = fit_font(&live_txt, 6.2 * k, (rect.width() - 6.0 * k).max(2.0));
        let h_live = if has_live { lf0 * 1.28 } else { 0.0 };
        let h_press = if has_trav { travel_font0 * 1.32 } else { 0.0 };
        // Vertical bands (round 16): red/blue travels return to the very
        // bottom (the driver template position); the live depth row gets
        // its own band directly above them so the travel distances no
        // longer drift upward; orig/binding share the top text zone.
        let bot = rect.bottom() - 1.0 * k;
        let press_bot = bot;
        let live_bot = press_bot - if has_trav { h_press + 0.7 * k } else { 0.0 };
        let text_bot = live_bot - if has_live { h_live + 0.7 * k } else { 0.0 };
        let top = rect.top() + 1.6 * k;
        let zone_h = (text_bot - top).max(1.0);
        let h_orig = if orig.is_empty() { 0.0 } else { of0 * 1.2 };
        let h_bind = if bind_txt.is_empty() { 0.0 } else { bf0 * 1.2 };
        // Spacer band between the original cap and the modified binding
        // (per request): keeps the two rows reading as separate lines.
        let h_gap = if orig.is_empty() || bind_txt.is_empty() {
            0.0
        } else {
            1.3 * k
        };
        // Fit the [orig, gap, bind] stack into the text zone; shrink both
        // fonts proportionally if the pair is too tall.
        let stack_fit = (zone_h / (h_orig + h_gap + h_bind + 0.3 * k).max(1.0))
            .min(1.0)
            .max(0.4);
        let (of, bf) = (of0 * stack_fit, bf0 * stack_fit);
        let (h_orig, h_bind, h_gapf) = (h_orig * stack_fit, h_bind * stack_fit, h_gap * stack_fit);
        let (orig_cy, bind_cy) = if bind_txt.is_empty() {
            // Free key: center the original inside the whole text zone.
            (top + (zone_h - h_orig) / 2.0 + h_orig / 2.0, 0.0)
        } else {
            (
                top + h_orig / 2.0,
                top + h_orig + h_gapf + h_bind / 2.0,
            )
        };
        if h_orig > 0.0 {
            painter.text(
                egui::pos2(rect.center().x, orig_cy),
                egui::Align2::CENTER_CENTER,
                orig,
                egui::FontId::proportional(of),
                if activated {
                    text_col
                } else {
                    text_col.gamma_multiply(0.82)
                },
            );
        }
        if h_bind > 0.0 {
            painter.text(
                egui::pos2(rect.center().x, bind_cy),
                egui::Align2::CENTER_CENTER,
                &bind_txt,
                egui::FontId::proportional(bf),
                text_col,
            );
        }
        // Live depth band, above the travel row (current travel in mm + %).
        if has_live {
            painter.text(
                egui::pos2(rect.center().x, live_bot),
                egui::Align2::CENTER_BOTTOM,
                &live_txt,
                egui::FontId::proportional(lf0),
                text_col,
            );
        }
        // Activation travels in the template's exact anchors/roles
        // (`d9f04973.js he/pe/ve` + `2a8cd9c4.js` template): press bottom-left
        // at x+3/y=39.5 (#FE4545), lift anchored at x=22 of the 42-unit box —
        // i.e. `right - 20k` so wide keys glue it to their right edge
        // (#0066FF). `pe()` shrinks the font as digits grow
        // (`8 * 4 / (precision+2)`), and we add a per-key fit guard that
        // shrinks both numbers until they fit side by side inside this key's
        // own width (the browser font was narrower than ours at small sizes —
        // that's why values overlapped before).
        let press = of_slot(&app.travel_press, slot);
        let lift = of_slot(&app.travel_lift, slot);
        if press.is_some() || lift.is_some() {
            let fmt = |t: Option<f32>| match t {
                Some(v) => format!("{v:.digits$}"),
                None => String::new(),
            };
            let (pt, lt) = (fmt(press), fmt(lift));
            // Disjoint per-key zones — the numbers can never overlap because
            // each shrinks only within its own span: press in
            // [left+3k, lift-anchor) and lift in (lift-anchor, right-2k];
            // when a value is alone its zone widens to the inner box. That
            // plus `pe()`'s 8*4/(precision+2) font rule keeps every key
            // readable at any window scale.
            let into = |txt: &str, zone: f32, font: f32| -> f32 {
                if txt.is_empty() || zone <= 2.0 {
                    return font;
                }
                let w0 = fit(txt, font);
                if w0 <= zone || font <= 3.0 {
                    font
                } else {
                    (font * zone / w0).max(3.0)
                }
            };
            let press_zone = if lift.is_some() {
                (rect.width() - 23.0 * k - 1.5 * k).max(2.0)
            } else {
                (rect.width() - 5.0 * k).max(2.0)
            };
            let lift_zone = if press.is_some() {
                (18.0 * k).max(2.0)
            } else {
                (rect.width() - 22.0 * k).max(2.0)
            };
            let (tfp, tfl) = (
                into(&pt, press_zone, travel_font0),
                into(&lt, lift_zone, travel_font0),
            );
            if let Some(t) = press {
                painter.text(
                    egui::pos2(rect.left() + 3.0 * k, press_bot),
                    egui::Align2::LEFT_BOTTOM,
                    format!("{t:.digits$}"),
                    egui::FontId::proportional(tfp),
                    PRESS_COL,
                );
            }
            if let Some(t) = lift {
                painter.text(
                    egui::pos2(rect.right() - 20.0 * k, press_bot),
                    egui::Align2::LEFT_BOTTOM,
                    format!("{t:.digits$}"),
                    egui::FontId::proportional(tfl),
                    LIFT_COL,
                );
            }
        }
        if selected {
            painter.rect_stroke(
                rect,
                round,
                egui::Stroke::new(2.0_f32, SELECT_COL),
                egui::StrokeKind::Outside,
            );
        }
        if resp.hovered() {
            if let Some(p) = resp.interact_pointer_pos() {
                if rect.contains(p) {
                    let fmt = |t: Option<f32>| match t {
                        Some(v) => format!("{v:.digits$}"),
                        None => String::from("-"),
                    };
                    hover_info = Some(format!(
                        "{label} (slot {}, {})\nBinding: {}\nPress {} mm \u{2022} Release \
                         {} mm \u{2022} Live {mm:.digits$} mm ({live_pct}%)",
                        slot.map(|s| s.to_string()).unwrap_or_else(|| "-".into()),
                        switches::switch_display(sw_code),
                        if binding.is_empty() { "default" } else { &binding },
                        fmt(of_slot(&app.travel_press, slot)),
                        fmt(of_slot(&app.travel_lift, slot)),
                    ));
                }
            }
        }
        if resp.clicked() && rect.contains(resp.interact_pointer_pos().unwrap_or(egui::Pos2::ZERO)) {
            if let Some(s) = slot {
                if let Some(i) = app.remap_entries.iter().position(|e| e.slot == s) {
                    select_entry(app, i);
                }
            }
            let _ = lid;
        }
    }
    if let Some(info) = hover_info {
        resp.on_hover_text(info);
    }
    // Live legend: switch colors (driver 轴体颜色字典) + travel value colors.
    ui.horizontal_wrapped(|ui| {
        for ((_, label), [r, g, b]) in switches::SK75_SWITCHES
            .iter()
            .zip(switches::SK75_SWITCH_COLORS.iter())
        {
            ui.label(
                egui::RichText::new("■ ")
                    .color(egui::Color32::from_rgb(*r, *g, *b))
                    .small(),
            );
            ui.label(egui::RichText::new(*label).small());
        }
    });
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("■ ").color(PRESS_COL).small());
        ui.label(egui::RichText::new("press activation").small());
        ui.label(egui::RichText::new("■ ").color(LIFT_COL).small());
        ui.label(egui::RichText::new("release activation").small());
        ui.label(
            egui::RichText::new("live % = of the 4.0 mm full stroke")
                .weak()
                .small(),
        );
        ui.label(
            egui::RichText::new("(key fill color = switch type, in this order)").weak(),
        );
    });
    ui.separator();
    binding_editor(ui, app);
}

/// Text color for a fill: dark on light/saturated switch colors, white on
/// dark ones (YIQ luminance; driver uses #3E3E3E-ish text on its dark keys
/// and white keycodes on colored fills — we pick per fill for readability).
fn text_on(fill: egui::Color32) -> egui::Color32 {
    let (r, g, b) = (fill.r(), fill.g(), fill.b());
    let yiq = 0.299 * f32::from(r) + 0.587 * f32::from(g) + 0.114 * f32::from(b);
    if yiq > 140.0 {
        egui::Color32::from_rgb(0x18, 0x18, 0x18)
    } else {
        egui::Color32::from_rgb(0xF4, 0xF4, 0xF4)
    }
}

/// Select a key entry and preseed the binding editors from its current binding.
fn select_entry(app: &mut App, i: usize) {
    app.remap_selected = Some(i);
    let Some(e) = app.remap_entries.get(i) else {
        return;
    };
    match &e.binding {
        keymap::Binding::Combo { skey, key, key2 } => {
            app.remap_bind_type = String::from("Key");
            // Preseed the exact triple so untouched entries write back
            // byte-identical bytes (e.g. Fn passthrough `[0,18,0,0]`).
            app.remap_combo = [*skey as u32, *key, *key2 as u32];
        }
        keymap::Binding::Disabled => app.remap_bind_type = String::from("Disabled"),
        keymap::Binding::Mouse { .. } => app.remap_bind_type = String::from("Mouse"),
        keymap::Binding::Function { .. } => app.remap_bind_type = String::from("Function"),
        keymap::Binding::Gamepad { .. } => app.remap_bind_type = String::from("Gamepad"),
        keymap::Binding::Macro { .. } => app.remap_bind_type = String::from("Macro"),
        _ => app.remap_bind_type = String::from("Key"),
    }
    if let Some(code) = app.switch_codes.get(e.slot).copied() {
        if let Some(pos) = switches::SK75_SWITCHES.iter().position(|(c, _)| *c == code) {
            app.selected_switch = pos;
        }
    }
}

/// Rebinding editor for the board-selected key. Clicking the key shows its
/// options here; every option is visible at a glance (no HID numbers to
/// type): standard-key grid from the driver's key tables
/// (`keyCodeToDisplayText` in `f01f4a92.js`; REA parent `ev_9da292dd...`),
/// plus mouse / media / gamepad / macro / disabled (cmds 10/138, Fn 16/144).
fn binding_editor(ui: &mut egui::Ui, app: &mut App) {
    let Some(i) = app.remap_selected else {
        ui.label("Click a key on the board to see its rebind options.");
        if !app.remap_msg.is_empty() {
            ui.small(&app.remap_msg);
        }
        return;
    };
    let Some(entry) = app.remap_entries.get(i).cloned() else {
        ui.label("Click a key on the board to see its rebind options.");
        return;
    };
    ui.heading(format!(
        "Key {} ({}) — current: {}",
        entry.slot,
        entry.default_name,
        entry.binding.short_label()
    ));
    ui.horizontal(|ui| {
        ui.label("Rebind to:");
        for b in ["Key", "Mouse", "Function", "Gamepad", "Macro", "Disabled"] {
            ui.selectable_value(&mut app.remap_bind_type, b.to_string(), b);
        }
    });
    match app.remap_bind_type.as_str() {
        "Disabled" => {
            ui.label("The slot will be disabled ([0,0,0,0]).");
        }
        "Mouse" => {
            let opts = keymap::mouse_options();
            ui.horizontal_wrapped(|ui| {
                for (idx, (label, _)) in opts.iter().enumerate() {
                    ui.selectable_value(&mut app.remap_mouse, idx, label);
                }
            });
        }
        "Function" => {
            let opts = keymap::function_options();
            ui.horizontal_wrapped(|ui| {
                for (idx, (label, _)) in opts.iter().enumerate() {
                    ui.selectable_value(&mut app.remap_function, idx, label);
                }
            });
        }
        "Gamepad" => {
            let opts = keymap::gamepad_options();
            ui.horizontal_wrapped(|ui| {
                for (idx, (label, _)) in opts.iter().enumerate() {
                    ui.selectable_value(&mut app.remap_gamepad, idx, label);
                }
            });
        }
        "Macro" => {
            ui.horizontal(|ui| {
                ui.label("Type");
                egui::ComboBox::from_id_salt("macroty")
                    .selected_text(&app.remap_macro_type)
                    .show_ui(ui, |ui| {
                        for m in ["repeat_times", "on_off", "touch_repeat"] {
                            ui.selectable_value(&mut app.remap_macro_type, m.to_string(), m);
                        }
                    });
                ui.add(
                    egui::DragValue::new(&mut app.remap_macro_idx)
                        .range(0..=15)
                        .prefix("slot "),
                );
            });
            ui.small("Assigns the macro slot; macro content editing lives in the macro editor.");
        }
        _ => {
            ui.label("Standard keys (click to add/remove — combo holds up to 3, driver limit):");
            let picked_names: Vec<String> = app
                .remap_combo
                .into_iter()
                .filter(|h| *h != 0)
                .map(matrix::hid_name)
                .collect();
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Combo ({}/{}): {}",
                    picked_names.len(),
                    keymap::MAX_COMBO_KEYS,
                    if picked_names.is_empty() {
                        String::from("pick keys below")
                    } else {
                        picked_names.join(" + ")
                    }
                ));
            });
            egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for h in keymap::STANDARD_KEYS {
                        let picked = app.remap_combo.contains(&h);
                        let label = matrix::arrow_glyph(h)
                            .map(str::to_string)
                            .unwrap_or_else(|| matrix::hid_name(h));
                        if ui
                            .selectable_label(picked, label)
                            .on_hover_text(matrix::hid_name(h))
                            .clicked()
                        {
                            // Toggle on click, capped at the driver limit —
                            // mirrors `handleClick_组合键` (`length>=3` return).
                            // New picks fill key, then key2, then skey.
                            if let Some(pos) =
                                app.remap_combo.iter().position(|x| *x == h)
                            {
                                app.remap_combo[pos] = 0;
                            } else if let Some(slot) = [1usize, 2, 0]
                                .into_iter()
                                .find(|i| app.remap_combo[*i] == 0)
                            {
                                app.remap_combo[slot] = h;
                            } else {
                                app.remap_msg = format!(
                                    "Combo holds at most {} keys (driver limit).",
                                    keymap::MAX_COMBO_KEYS
                                );
                            }
                        }
                    }
                });
            });
        }
    }
    let current_code = app.switch_codes.get(entry.slot).copied().unwrap_or(0);
    ui.horizontal(|ui| {
        ui.label("Switch type");
        egui::ComboBox::from_id_salt("switch")
            .selected_text(
                switches::SK75_SWITCHES
                    .get(app.selected_switch)
                    .map(|(_, label)| *label)
                    .unwrap_or("Unknown"),
            )
            .show_ui(ui, |ui| {
                for (idx, (_, label)) in switches::SK75_SWITCHES.iter().enumerate() {
                    ui.selectable_value(&mut app.selected_switch, idx, *label);
                }
            });
        ui.small(format!(
            "current: {}",
            switches::switch_display(current_code)
        ));
        if ui.button("Write switch").clicked() {
            app.apply_switch();
        }
    });
    ui.horizontal(|ui| {
        if ui.button("Write key").clicked() {
            app.apply_remap();
        }
        if ui
            .button("Reset")
            .on_hover_text("Restore this key's factory default binding on the current layer")
            .clicked()
        {
            app.reset_selected_key();
        }
        ui.label(&app.remap_msg);
    });
    if !app.switch_msg.is_empty() {
        ui.small(&app.switch_msg);
    }
}

fn calibration_tab(ui: &mut egui::Ui, app: &mut App) {
    ui.heading("Travel calibration");
    ui.label("Same sequence as the original driver: min-cal on → 2 s → min-cal off → max-cal on. Keep hands off the switches and stay wired.");
    if ui
        .add_enabled(!app.cal_busy, egui::Button::new("Start calibration"))
        .clicked()
    {
        app.cal_busy = true;
        app.cal_msg = String::from("Running min/max calibration…");
        match hid::open_sk75(app.vid, app.pid, app.path_opt(), None) {
            Ok(dev) => match dev.calibrate_start() {
                Ok(()) => {
                    app.cal_msg = String::from(
                        "Max-cal armed. Wait ~8 s untouched, then press “Finish”.",
                    );
                }
                Err(e) => {
                    app.cal_msg = format!("Calibration failed: {e:#}");
                    app.cal_busy = false;
                }
            },
            Err(e) => {
                app.cal_msg = format!("{e:#}");
                app.cal_busy = false;
            }
        }
    }
    if app.cal_busy && ui.button("Finish (max-cal off)").clicked() {
        match hid::open_sk75(app.vid, app.pid, app.path_opt(), None) {
            Ok(dev) => match dev.calibrate_stop() {
                Ok(()) => app.cal_msg = String::from("Calibration finished. Re-read analog to verify rest ≈ 0."),
                Err(e) => app.cal_msg = format!("Finish failed: {e:#}"),
            },
            Err(e) => app.cal_msg = format!("{e:#}"),
        }
        app.cal_busy = false;
    }
    ui.label(&app.cal_msg);
}

/// Load a system Nerd Font into egui so the arrow glyphs
/// (`matrix::arrow_glyph`, Font Awesome codepoints) render as icons.
/// Falls back to egui's built-in fonts when none is installed.
fn install_nerd_font(ctx: &egui::Context) {
    let mut cands: Vec<std::path::PathBuf> = Vec::new();
    let mut roots = vec![
        std::path::PathBuf::from("/usr/share/fonts"),
        std::path::PathBuf::from("/usr/local/share/fonts"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        let home = std::path::PathBuf::from(home);
        roots.push(home.join(".local/share/fonts"));
        roots.push(home.join(".fonts"));
    }
    fn visit(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                visit(&p, out);
            } else if let Some(n) = p.file_name().and_then(|n| n.to_str()) {
                let lower = n.to_lowercase();
                if lower.contains("nerd") && (lower.ends_with(".ttf") || lower.ends_with(".otf")) {
                    out.push(p);
                }
            }
        }
    }
    for r in &roots {
        visit(r, &mut cands);
    }
    cands.sort_by_key(|p| {
        let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
        (!n.contains("regular"), !n.contains("medium"), n.len())
    });
    let Some(path) = cands.into_iter().next() else {
        return;
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "nerd".to_owned(),
        egui::FontData::from_owned(bytes).into(),
    );
    for fam in [
        egui::FontFamily::Proportional,
        egui::FontFamily::Monospace,
    ] {
        fonts
            .families
            .entry(fam)
            .or_default()
            .insert(0, "nerd".to_owned());
    }
    ctx.set_fonts(fonts);
}

fn main() -> eframe::Result<()> {
    env_logger::init();
    eframe::run_native(
        "SK75 TMR",
        eframe::NativeOptions::default(),
        Box::new(|cc| {
            install_nerd_font(&cc.egui_ctx);
            Ok(Box::new(App::new()))
        }),
    )
}
