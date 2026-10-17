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
        let mat = match self.info.device_id {
            Some(3804) => &matrix::MATRIX_3804,
            _ => &matrix::MATRIX_2518,
        };
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
        let mat = match self.info.device_id {
            Some(3804) => &matrix::MATRIX_3804,
            _ => &matrix::MATRIX_2518,
        };
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
        let mat = match self.info.device_id {
            Some(3804) => &matrix::MATRIX_3804,
            _ => &matrix::MATRIX_2518,
        };
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

    /// Re-read bindings + switches after a write while keeping `slot` selected.
    fn refresh_keep_selected(&mut self, slot: usize, written: &str) {
        self.read_remap();
        self.read_switches();
        self.remap_selected = self.remap_entries.iter().position(|e| e.slot == slot);
        self.remap_msg = format!("{written}. Verified by re-read.");
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
        match hid::open_sk75(self.vid, self.pid, self.path_opt(), None) {
            Ok(dev) => match dev.write_switch_type(slot, code) {
                Ok(()) => {
                    if let Some(c) = self.switch_codes.get_mut(slot) {
                        *c = code;
                    }
                    self.switch_msg =
                        format!("Slot {slot} switch -> {label} ({code}). Re-read to verify.");
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
        let first = app.last_layer.is_none();
        app.last_layer = Some(cur_layer);
        if !first {
            app.read_remap();
            app.read_switches();
            app.snapshot_preview();
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
    let mat = match app.info.device_id {
        Some(3804) => &matrix::MATRIX_3804,
        _ => &matrix::MATRIX_2518,
    };
    // slot -> (mm, binding label, switch label)
    let live: std::collections::HashMap<usize, f32> =
        app.bars.iter().map(|b| (b.slot, b.mm)).collect();
    let scale = layout::CANVAS_W.min(900.0) / layout::CANVAS_W;
    let (ox, oy) = (8.0, 8.0);
    let h = layout::CANVAS_H * scale + 16.0;
    let (resp, painter) =
        ui.allocate_painter(egui::Vec2::new(layout::CANVAS_W * scale + 16.0, h), egui::Sense::click());
    for (lid, hid, x, y, w, hgt, label) in layout::KEYS {
        let rect = egui::Rect::from_min_size(
            resp.rect.min + egui::Vec2::new(ox + x * scale, oy + y * scale),
            egui::Vec2::new(w * scale, hgt * scale),
        );
        let slot = layout::slot_for(hid, mat);
        let mm = slot.and_then(|s| live.get(&s).copied()).unwrap_or(0.0);
        let depth = (mm / 3.4).clamp(0.0, 1.0);
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
        let fill = egui::Color32::from_rgb(
            40 + (70.0 * depth) as u8,
            120 + (60.0 * depth) as u8,
            200,
        );
        painter.rect_filled(rect, 4.0 * scale as f32, fill);
        if selected {
            painter.rect_stroke(
                rect,
                4.0 * scale as f32,
                egui::Stroke::new(2.0, egui::Color32::YELLOW),
                egui::StrokeKind::Outside,
            );
        }
        // Same font/glyphs as the rebinding picker: arrow keys render as
        // Nerd-Font icons instead of asset arrows that may miss a glyph.
        let face: &str = matrix::arrow_glyph(hid).unwrap_or(label);
        painter.text(
            rect.center_top() + egui::Vec2::new(0.0, 3.0),
            egui::Align2::CENTER_TOP,
            face,
            egui::FontId::proportional(11.0),
            egui::Color32::WHITE,
        );
        let sub: String = if mm > 0.0 {
            format!("{mm:.2}")
        } else if binding.is_empty() {
            String::new()
        } else {
            binding.chars().take(10).collect()
        };
        painter.text(
            rect.center_bottom() - egui::Vec2::new(0.0, 3.0),
            egui::Align2::CENTER_BOTTOM,
            sub,
            egui::FontId::proportional(9.0),
            egui::Color32::WHITE,
        );
        if resp.clicked() && rect.contains(resp.interact_pointer_pos().unwrap_or(egui::Pos2::ZERO)) {
            if let Some(s) = slot {
                if let Some(i) = app.remap_entries.iter().position(|e| e.slot == s) {
                    select_entry(app, i);
                }
            }
            let _ = lid;
        }
    }
    ui.small("Tip: start the live overlay, press switches, and watch depths rise on the board.");
    ui.separator();
    binding_editor(ui, app);
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
                if ui.button("Clear").clicked() {
                    app.remap_combo = [0, 0, 0];
                }
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
    ui.horizontal(|ui| {
        ui.label("Switch type");
        egui::ComboBox::from_id_salt("switch")
            .selected_text(switches::switch_display(
                app.switch_codes
                    .get(entry.slot)
                    .copied()
                    .unwrap_or(0),
            ))
            .show_ui(ui, |ui| {
                for (idx, (_, label)) in switches::SK75_SWITCHES.iter().enumerate() {
                    ui.selectable_value(&mut app.selected_switch, idx, *label);
                }
            });
        if ui.button("Write switch").clicked() {
            app.apply_switch();
        }
    });
    ui.horizontal(|ui| {
        if ui.button("Write key").clicked() {
            app.apply_remap();
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
