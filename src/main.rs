//! `sk75-tmr` — Linux Rust recreation of the WOMIER SK75 TMR driver.
//!
//! Focus: recognize the analog (TMR / Hall-style) values of every switch
//! via vendor HID feature reports, without Windows / Electron / iot_manager.

mod device_info;
mod hid;
mod keymap;
mod layout;
mod matrix;
mod proto;
mod switches;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(name = "sk75-tmr", version, about = "WOMIER SK75 TMR analog driver for Linux")]
struct Cli {
    #[arg(long, default_value_t = proto::VID_RONGYUAN, help = "USB vendor ID")]
    vid: u16,
    #[arg(long, default_value_t = proto::PID_SK75_TMR, help = "USB product ID")]
    pid: u16,
    #[arg(long, help = "hidraw path, e.g. /dev/hidraw3 (auto-picks vendor page otherwise)")]
    path: Option<String>,
    #[arg(long, help = "HID interface number filter")]
    iface: Option<i32>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List matching HID devices.
    List,
    /// Identify device: whoAmI id, USB version, precision/scale.
    Info,
    /// Poll live analog travel (op 254) once or continuously.
    ///
    /// Prints per-key `slot hid name raw mm` lines. Only keys with
    /// raw > threshold (or all with --all) are shown in loop mode.
    Monitor {
        #[arg(long, default_value_t = 0, help = "0 = run forever, else seconds")]
        seconds: u64,
        #[arg(long, default_value_t = 1, help = "Show keys with raw >= threshold")]
        threshold: u16,
        #[arg(long, help = "Show all keys every poll, including untouched")]
        all: bool,
        #[arg(long, help = "Also read op 255 dynamic values")]
        dynamic: bool,
        #[arg(long, help = "Matrix variant: 2518 or 3804 (default: auto by device id)")]
        matrix: Option<u32>,
    },
    /// Run travel calibration (min then max), like the driver travel calibration.
    Calibrate {
        #[arg(long, default_value_t = 8, help = "Seconds to wait for max-cal settle")]
        settle: u64,
    },
    /// Enable/disable unsolicited magnetism streaming (cmd 27).
    Stream { #[arg(long)] on: bool },
    /// Key rebinding: read/write the keymap layer (cmds 10/138, Fn 16/144).
    Keymap {
        #[command(subcommand)]
        action: KeymapAction,
    },
    /// Switch (axis) type selection per key (op 252).
    Switch {
        #[command(subcommand)]
        action: SwitchAction,
    },
    /// Offline analog-decode demo (no hardware): decode a synthetic op254
    /// blob and print mm values, proving the TMR conversion path.
    Demo,
}

#[derive(Subcommand)]
enum KeymapAction {
    /// Dump a keymap layer: slot, default key, current binding.
    Read {
        #[arg(long, default_value_t = 0, help = "Profile/layer 0..3")]
        profile: u8,
        #[arg(long, help = "Fn layer instead: win or mac")]
        r#fn: Option<String>,
    },
    /// Rebind one key slot (single-key write, mirrors setKeyConfigSimple).
    Write {
        #[arg(long, default_value_t = 0, help = "Profile/layer 0..3")]
        profile: u8,
        #[arg(long, help = "Matrix slot 0..127 (see read)")]
        slot: usize,
        #[arg(long, num_args(1..=3), value_name = "HID", help = "Combo HIDs 4..231 in press order, e.g. --hid 224 6 for LCtrl+C (max 3, driver limit)")]
        hid: Vec<u32>,
        #[arg(long, help = "Disable the key")]
        disable: bool,
        #[arg(long, help = "Mouse button: Left|Right|Middle|Back|Forward|WheelUp|WheelDown")]
        mouse: Option<String>,
        #[arg(long, help = "Function/media label, e.g. 'Volume+' (see read)")]
        function: Option<String>,
        #[arg(long, help = "Gamepad button label, e.g. 'A'")]
        gamepad: Option<String>,
        #[arg(long, help = "Macro slot 0..15 as TYPE:IDX, e.g. repeat_times:2")]
        r#macro: Option<String>,
        #[arg(long, help = "Raw quad a,b,c,d (advanced: Snap/MDS/MT/Recoil)")]
        quad: Option<String>,
    },
}

#[derive(Subcommand)]
enum SwitchAction {
    /// Read per-slot switch codes for the 128 matrix slots.
    Read,
    /// Set one slot's switch type by English label (see read).
    Write {
        #[arg(long, help = "Matrix slot 0..127")]
        slot: usize,
        #[arg(long, help = "Switch label, e.g. 'POM Magnet-King'")]
        switch: String,
    },
}

fn pick_matrix(device_id: Option<u32>, force: Option<u32>) -> &'static [u8; 512] {
    if let Some(3804) = force {
        return &matrix::MATRIX_3804;
    }
    if let Some(2518) = force {
        return &matrix::MATRIX_2518;
    }
    match device_id {
        Some(3804) => &matrix::MATRIX_3804,
        _ => &matrix::MATRIX_2518, // default: id 2518 layout (both are 128 slots)
    }
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::List => {
            let devs = hid::enumerate_sk75(cli.vid, cli.pid)?;
            if devs.is_empty() {
                println!("no {:04x}:{:04x} HID device found", cli.vid, cli.pid);
                println!("hint: check cable (use wired mode), udev rules, and that no Windows VM holds the device");
            }
            for d in devs {
                println!(
                    "path={} iface={} usage={:#x} page={:#x} '{}' '{}'",
                    d.path().to_string_lossy(),
                    d.interface_number(),
                    d.usage(),
                    d.usage_page(),
                    d.manufacturer_string().unwrap_or(""),
                    d.product_string().unwrap_or("")
                );
            }
        }
        Cmd::Info => {
            let info =
                device_info::read_keyboard_info(cli.vid, cli.pid, cli.path.as_deref(), cli.iface);
            println!("display: {}", info.display_name);
            println!("path: {}", info.path);
            println!("vid:pid {} ({})", info.vid_pid_hex(), info.vid_pid_dec());
            println!("product: '{}' manufacturer: '{}'", info.product, info.manufacturer);
            println!("device id: {}", info.device_id_text());
            println!("variant: {}", info.variant);
            println!("usb version: {}", info.usb_version_text());
            println!("precision: {}", info.precision_text());
            println!("report rate: {}", info.report_rate_text());
            println!("keys: {}", info.key_count);
            if let Some(e) = info.error {
                println!("note: {e}");
            }
        }
        Cmd::Monitor {
            seconds,
            threshold,
            all,
            dynamic,
            matrix,
        } => {
            let k = hid::open_sk75(cli.vid, cli.pid, cli.path.as_deref(), cli.iface)?;
            let id = k.device_id().unwrap_or(None);
            let scale = k.travel_scale().unwrap_or(100);
            let mat = pick_matrix(id, matrix);
            let keys = matrix::populated_keys(mat);
            // Original driver: getJiaoZhunXinXiLoop re-polls with no sleep;
            // each poll is 4x commonMsg(254, SEND 1ms, READ 1ms). Always run
            // back-to-back at HID round-trip speed (measured ~83 Hz).
            println!("# SK75 TMR live analog: op254 travel (op255 with --dynamic), scale={scale}/mm, max-speed 1ms pacing");
            println!("# slot hid name raw mm [dyn_raw]");
            let deadline = if seconds == 0 {
                None
            } else {
                Some(Instant::now() + Duration::from_secs(seconds))
            };
            let start = Instant::now();
            let mut polls: u64 = 0;
            loop {
                let travel = k.live_travel_raw().context("op254 live_travel read failed")?;
                let dynv = if dynamic {
                    Some(k.live_dynamic_raw().context("op255 live_dynamic read failed")?)
                } else {
                    None
                };
                polls += 1;
                let mut shown = 0;
                for &(slot, h) in &keys {
                    let raw = travel.get(slot).copied().unwrap_or(0);
                    if !all && raw < threshold {
                        continue;
                    }
                    let mm = proto::raw_to_mm(raw, scale);
                    match &dynv {
                        Some(d) => println!(
                            "{slot:3} {h:3} {:8} {raw:5} {mm:5.3} {}",
                            matrix::hid_name(h),
                            d.get(slot).copied().unwrap_or(0)
                        ),
                        None => println!("{slot:3} {h:3} {:8} {raw:5} {mm:5.3}", matrix::hid_name(h)),
                    }
                    shown += 1;
                }
                if shown == 0 {
                    println!("(no key >= threshold {threshold}; press a switch)");
                }
                println!("---");
                if let Some(dl) = deadline {
                    if Instant::now() >= dl {
                        let hz = polls as f64 / start.elapsed().as_secs_f64().max(0.001);
                        println!("# {polls} polls, {hz:.0} Hz");
                        break;
                    }
                }
            }
        }
        Cmd::Calibrate { settle } => {
            let k = hid::open_sk75(cli.vid, cli.pid, cli.path.as_deref(), cli.iface)?;
            println!("starting travel calibration (min-cal on, 2 s, min-cal off, max-cal on) ...");
            println!("leave all switches untouched, keep the keyboard wired.");
            k.calibrate_start()?;
            println!("max-cal armed; waiting {settle}s, then disarming ...");
            std::thread::sleep(Duration::from_secs(settle));
            k.calibrate_stop()?;
            println!("calibration finished (max-cal off). Re-run `monitor` to verify travel returns to ~0 at rest.");
        }
        Cmd::Stream { on } => {
            let k = hid::open_sk75(cli.vid, cli.pid, cli.path.as_deref(), cli.iface)?;
            k.set_magnetism_report(on)?;
            println!("magnetism streaming (cmd 27) -> {on}");
        }
        Cmd::Keymap { action } => match action {
            KeymapAction::Read { profile, r#fn } => {
                if profile > 3 {
                    anyhow::bail!("profile must be 0..3");
                }
                let k = hid::open_sk75(cli.vid, cli.pid, cli.path.as_deref(), cli.iface)?;
                let id = k.device_id().unwrap_or(None);
                let mat = pick_matrix(id, None);
                let blob = match r#fn.as_deref() {
                    Some(s) if s.eq_ignore_ascii_case("win") => {
                        k.read_fn_keymap(profile, keymap::FnSys::Win)?
                    }
                    Some(s) if s.eq_ignore_ascii_case("mac") => {
                        k.read_fn_keymap(profile, keymap::FnSys::Mac)?
                    }
                    Some(other) => anyhow::bail!("--fn must be win or mac, got {other}"),
                    None => k.read_keymap(profile)?,
                };
                let entries = keymap::decode_keymap(
                    &blob,
                    mat.try_into().map_err(|_| anyhow::anyhow!("matrix len"))?,
                );
                let what = match r#fn.as_deref() {
                    Some(s) => format!("fn {s} profile {profile}"),
                    None => format!("profile {profile}"),
                };
                println!("# keymap {what}: slot default current changed");
                for e in entries {
                    println!(
                        "{:3} {:>8} {:>24} {}",
                        e.slot,
                        e.default_name,
                        e.binding.short_label(),
                        if e.changed { "*" } else { "" }
                    );
                }
            }
            KeymapAction::Write {
                profile,
                slot,
                hid,
                disable,
                mouse,
                function,
                gamepad,
                r#macro,
                quad,
            } => {
                if profile > 3 {
                    anyhow::bail!("profile must be 0..3");
                }
                let mut set = 0;
                let mut binding = None;
                if !hid.is_empty() {
                    set += 1;
                    binding = Some(keymap::combo_from_keys(&hid).ok_or_else(|| {
                        anyhow::anyhow!(
                            "combo needs 1..={} HIDs fitting one byte each",
                            keymap::MAX_COMBO_KEYS
                        )
                    })?);
                }
                if disable {
                    set += 1;
                    binding = Some(keymap::Binding::Disabled);
                }
                if let Some(m) = mouse {
                    set += 1;
                    let (_, q) = keymap::mouse_options()
                        .into_iter()
                        .find(|(l, _)| l.eq_ignore_ascii_case(&m))
                        .ok_or_else(|| anyhow::anyhow!("unknown mouse button {m}"))?;
                    binding = Some(keymap::Binding::Mouse { label: m, quad: q });
                }
                if let Some(f) = function {
                    set += 1;
                    let (_, q) = keymap::function_options()
                        .into_iter()
                        .find(|(l, _)| l.eq_ignore_ascii_case(&f))
                        .ok_or_else(|| anyhow::anyhow!("unknown function {f}"))?;
                    binding = Some(keymap::Binding::Function { label: f, quad: q });
                }
                if let Some(g) = gamepad {
                    set += 1;
                    let (_, q) = keymap::gamepad_options()
                        .into_iter()
                        .find(|(l, _)| l.eq_ignore_ascii_case(&g))
                        .ok_or_else(|| anyhow::anyhow!("unknown gamepad {g}"))?;
                    binding = Some(keymap::Binding::Gamepad { label: g, quad: q });
                }
                if let Some(m) = r#macro {
                    set += 1;
                    let (ty, idx) = m.split_once(':').ok_or_else(|| {
                        anyhow::anyhow!("--macro must be TYPE:IDX, e.g. repeat_times:2")
                    })?;
                    binding = Some(keymap::Binding::Macro {
                        macro_type: ty.to_string(),
                        index: idx.parse()?,
                    });
                }
                if let Some(q) = quad {
                    set += 1;
                    let nums: Vec<u8> =
                        q.split(',').map(|s| s.trim().parse()).collect::<Result<_, _>>()?;
                    if nums.len() != 4 {
                        anyhow::bail!("--quad must be a,b,c,d");
                    }
                    binding = Some(keymap::Binding::Unknown([nums[0], nums[1], nums[2], nums[3]]));
                }
                if set != 1 {
                    anyhow::bail!("pass exactly one of --hid/--disable/--mouse/--function/--gamepad/--macro/--quad");
                }
                let b = binding.unwrap();
                let k = hid::open_sk75(cli.vid, cli.pid, cli.path.as_deref(), cli.iface)?;
                k.write_key_simple(profile, slot, b.quad())?;
                println!(
                    "slot {slot} profile {profile} -> {} [{},{},{},{}]",
                    b.short_label(),
                    b.quad()[0],
                    b.quad()[1],
                    b.quad()[2],
                    b.quad()[3]
                );
            }
        },
        Cmd::Switch { action } => match action {
            SwitchAction::Read => {
                let k = hid::open_sk75(cli.vid, cli.pid, cli.path.as_deref(), cli.iface)?;
                let codes = k.read_switch_types_raw()?;
                println!("# switch types: slot code label");
                for (slot, code) in codes.iter().enumerate() {
                    println!("{slot:3} {code:3} {}", switches::switch_display(*code));
                }
            }
            SwitchAction::Write { slot, switch } => {
                let code = switches::switch_code(&switch).ok_or_else(|| {
                    anyhow::anyhow!("unknown switch '{switch}' (see switch read)")
                })?;
                let k = hid::open_sk75(cli.vid, cli.pid, cli.path.as_deref(), cli.iface)?;
                k.write_switch_type(slot, code)?;
                println!("slot {slot} -> {} ({code})", switches::switch_label(code));
            }
        },
        Cmd::Demo => {
            // Synthetic op254 blob: 128 LE u16, Esc=0, A=150 (1.5mm@100), Space=330 (3.3mm max).
            let mut blob = vec![0u8; 256];
            let put = |slot: usize, raw: u16, b: &mut Vec<u8>| {
                b[slot * 2] = (raw & 0xFF) as u8;
                b[slot * 2 + 1] = (raw >> 8) as u8;
            };
            put(0, 0, &mut blob);
            put(36, 150, &mut blob);
            put(52, 330, &mut blob);
            let vals = proto::decode_u16le_list(&blob);
            let scale = 100u32;
            let mat = &matrix::MATRIX_2518;
            let keys = matrix::populated_keys(mat);
            println!("# demo: synthetic op254 -> mm (scale={scale}/mm)");
            for &(slot, h) in &keys {
                let raw = vals[slot];
                if raw == 0 {
                    continue;
                }
                println!(
                    "{slot:3} {h:3} {:8} {raw:5} {:.3}mm",
                    matrix::hid_name(h),
                    proto::raw_to_mm(raw, scale)
                );
            }
        }
    }
    Ok(())
}
