//! Resident part: tray icon + global hotkey. On the hotkey it grabs the monitor
//! under the cursor *immediately* (before anything is drawn) and launches the
//! overlay process with that frozen frame.

use crate::autostart;
use crate::frame::{self, Frame};
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{Code, HotKey},
};
use std::process::{Child, Command};
use std::str::FromStr;
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

enum UserEvent {
    Hotkey(GlobalHotKeyEvent),
    Menu(MenuEvent),
}

#[cfg(windows)]
pub fn set_dpi_aware() {
    use windows_sys::Win32::UI::HiDpi::{
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
    };
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

#[cfg(not(windows))]
pub fn set_dpi_aware() {}

pub fn message_box(text: &str) {
    rfd::MessageDialog::new()
        .set_title("Cutline")
        .set_level(rfd::MessageLevel::Error)
        .set_description(text)
        .show();
}

/// Config lives in `<config dir>/Cutline/config.txt`, lines like `hotkey = F1`.
fn config_path() -> Option<std::path::PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(std::path::PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
    };
    base.map(|b| b.join("Cutline").join("config.txt"))
}

fn config_hotkey() -> String {
    // The env var wins; handy for trying a key without touching the config file.
    if let Ok(h) = std::env::var("CUTLINE_HOTKEY") {
        if !h.trim().is_empty() {
            return h.trim().to_string();
        }
    }
    if let Some(p) = config_path() {
        if let Ok(text) = std::fs::read_to_string(&p) {
            for line in text.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    if k.trim().eq_ignore_ascii_case("hotkey") && !v.trim().is_empty() {
                        return v.trim().to_string();
                    }
                }
            }
        } else if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
            let _ = std::fs::write(&p, "# Cutline settings\n# Examples: F1, F2, ctrl+shift+KeyA\nhotkey = F1\n");
        }
    }
    "F1".to_string()
}

fn tray_icon_image() -> Icon {
    // 32x32: blue rounded square with white crop corners.
    let n = 32usize;
    let mut px = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let (cx, cy) = (x as f32 - 15.5, y as f32 - 15.5);
            let inside = cx.abs() <= 14.0 && cy.abs() <= 14.0;
            let i = (y * n + x) * 4;
            if inside {
                px[i..i + 4].copy_from_slice(&[0x1e, 0x88, 0xe5, 0xff]);
                let (ax, ay) = (x as i32, y as i32);
                let edge = |v: i32| (8..=10).contains(&v) || (21..=23).contains(&v);
                let span = |v: i32| (8..=14).contains(&v) || (17..=23).contains(&v);
                if (edge(ax) && span(ay)) || (edge(ay) && span(ax)) {
                    px[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
                }
            }
        }
    }
    Icon::from_rgba(px, n as u32, n as u32).expect("icon")
}

/// Only one resident instance per user. The lock is released by the OS when we exit.
fn single_instance() -> Option<std::fs::File> {
    let user = std::env::var("USERNAME").or_else(|_| std::env::var("USER")).unwrap_or_default();
    let path = std::env::temp_dir().join(format!("cutline-{user}.lock"));
    let file = std::fs::OpenOptions::new().create(true).write(true).truncate(false).open(path).ok()?;
    file.try_lock().ok()?;
    Some(file)
}

pub fn run() {
    let Some(_lock) = single_instance() else { return };

    let hotkey_text = config_hotkey();
    let hotkey = HotKey::from_str(&hotkey_text)
        .or_else(|_| HotKey::from_str("F1"))
        .unwrap_or_else(|_| HotKey::new(None, Code::F1));

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    // Wake the event loop whenever the hotkey or a tray menu item fires.
    let proxy = event_loop.create_proxy();
    GlobalHotKeyEvent::set_event_handler(Some(move |e| {
        let _ = proxy.send_event(UserEvent::Hotkey(e));
    }));
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |e| {
        let _ = proxy.send_event(UserEvent::Menu(e));
    }));

    let manager = match GlobalHotKeyManager::new() {
        Ok(m) => m,
        Err(e) => {
            message_box(&format!("Could not start the hotkey service: {e}"));
            return;
        }
    };
    if let Err(e) = manager.register(hotkey) {
        message_box(&format!(
            "Could not register the hotkey \"{hotkey_text}\": {e}\n\nAnother program (Snipaste?) probably already uses it. \
             Quit that program, or change `hotkey` in the Cutline config file."
        ));
        return;
    }
    let hotkey_id = hotkey.id();

    let capture_item = MenuItem::new(format!("Capture ({hotkey_text})"), true, None);
    // Ticked from what the system has, not from config.txt (see autostart.rs).
    let mut autostart_on = autostart::is_enabled();
    let autostart_item = CheckMenuItem::new(autostart::LABEL, true, autostart_on, None);
    let quit_item = MenuItem::new("Quit", true, None);
    let capture_id = capture_item.id().clone();
    let autostart_id = autostart_item.id().clone();
    let quit_id = quit_item.id().clone();

    let mut tray: Option<TrayIcon> = None;
    let mut child: Option<Child> = None;
    let tooltip = format!("Cutline - {hotkey_text}");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        let _keep = (&manager, &_lock);
        let mut want_capture = false;
        match event {
            // Tray icons must be created once the event loop is running (required on macOS).
            Event::NewEvents(StartCause::Init) => {
                let menu = Menu::new();
                let _ = menu.append_items(&[
                    &capture_item,
                    &PredefinedMenuItem::separator(),
                    &autostart_item,
                    &PredefinedMenuItem::separator(),
                    &quit_item,
                ]);
                match TrayIconBuilder::new()
                    .with_menu(Box::new(menu))
                    .with_tooltip(tooltip.clone())
                    .with_icon(tray_icon_image())
                    .build()
                {
                    Ok(t) => tray = Some(t),
                    // e.g. a Linux desktop without a tray host: the hotkey still works.
                    Err(e) => eprintln!("cutline: no tray icon: {e}"),
                }
            }
            Event::UserEvent(UserEvent::Hotkey(ev)) => {
                if ev.id == hotkey_id && ev.state == HotKeyState::Pressed {
                    want_capture = true;
                }
            }
            Event::UserEvent(UserEvent::Menu(ev)) => {
                if ev.id == quit_id {
                    tray.take();
                    *control_flow = ControlFlow::Exit;
                } else if ev.id == capture_id {
                    want_capture = true;
                } else if ev.id == autostart_id {
                    // Flip what the item showed (the menu may have flipped its tick by itself
                    // already), then show what the system has now.
                    if let Err(e) = autostart::set_enabled(!autostart_on) {
                        message_box(&format!("Could not change \"{}\": {e}", autostart::LABEL));
                    }
                    autostart_on = autostart::is_enabled();
                    autostart_item.set_checked(autostart_on);
                }
            }
            _ => {}
        }
        if want_capture {
            // One overlay at a time.
            let busy = match child.as_mut() {
                Some(c) => matches!(c.try_wait(), Ok(None)),
                None => false,
            };
            if !busy {
                child = spawn_capture();
            }
        }
    });
}

/// `cutline --now`: capture and show the overlay, then exit.
pub fn capture_now() {
    if let Some(mut c) = spawn_capture() {
        let _ = c.wait();
    }
}

/// One monitor's pixels and geometry in the platform's own units: pixels on Windows and Linux,
/// points on macOS. `grab` turns this into the logical-point `Frame` the overlay expects.
struct Raw {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    dpi_scale: f32,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    /// `[x, y, w, h]`, topmost first.
    windows: Vec<[f32; 4]>,
}

#[cfg(windows)]
fn cursor_pos() -> Option<(i32, i32)> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut p = POINT { x: 0, y: 0 };
    (unsafe { GetCursorPos(&mut p) } != 0).then_some((p.x, p.y))
}

#[cfg(target_os = "macos")]
fn cursor_pos() -> Option<(i32, i32)> {
    match mouse_position::mouse_position::Mouse::get_mouse_position() {
        mouse_position::mouse_position::Mouse::Position { x, y } => Some((x, y)),
        mouse_position::mouse_position::Mouse::Error => None,
    }
}

/// Windows and macOS: xcap.
#[cfg(not(target_os = "linux"))]
fn grab_raw() -> Option<Raw> {
    let (cx, cy) = cursor_pos().unwrap_or((0, 0));
    // Fall back to the first monitor if the cursor position is unavailable.
    let mon = match xcap::Monitor::from_point(cx, cy) {
        Ok(m) => m,
        Err(_) => xcap::Monitor::all().ok()?.into_iter().next()?,
    };
    let img = mon.capture_image().ok()?;
    let dpi_scale = mon.scale_factor().unwrap_or(1.0).max(0.5);

    let mut windows = Vec::new();
    if let Ok(list) = xcap::Window::all() {
        for win in list {
            if win.is_minimized().unwrap_or(true) {
                continue;
            }
            let (Ok(wx), Ok(wy), Ok(ww), Ok(wh)) = (win.x(), win.y(), win.width(), win.height()) else { continue };
            if ww < 24 || wh < 24 {
                continue;
            }
            if win.title().map(|t| t.is_empty()).unwrap_or(true) {
                continue;
            }
            windows.push([wx as f32, wy as f32, ww as f32, wh as f32]);
        }
    }
    Some(Raw {
        x: mon.x().ok()? as f32,
        y: mon.y().ok()? as f32,
        w: mon.width().ok()? as f32,
        h: mon.height().ok()? as f32,
        dpi_scale,
        width: img.width(),
        height: img.height(),
        rgba: img.into_raw(),
        windows,
    })
}

/// Linux: talk to the X server directly (see capture_x11.rs).
#[cfg(target_os = "linux")]
fn grab_raw() -> Option<Raw> {
    if crate::capture_x11::is_wayland() {
        message_box(
            "Cutline can't capture the screen on Wayland yet.\n\n\
             Log in with an X11 session (for example \"Ubuntu on Xorg\") to use it. \
             Setting CUTLINE_FORCE_X11=1 captures only X11 windows through XWayland.",
        );
        return None;
    }
    match crate::capture_x11::grab() {
        Ok(c) => Some(Raw {
            x: c.x as f32,
            y: c.y as f32,
            w: c.w as f32,
            h: c.h as f32,
            dpi_scale: c.dpi_scale,
            width: c.w,
            height: c.h,
            rgba: c.rgba,
            windows: c.windows.iter().map(|w| [w[0] as f32, w[1] as f32, w[2] as f32, w[3] as f32]).collect(),
        }),
        Err(e) => {
            eprintln!("cutline: screen capture failed: {e}");
            None
        }
    }
}

fn grab() -> Option<Frame> {
    let raw = grab_raw()?;
    // Windows and Linux report pixels; macOS reports points already.
    let to_logical = |v: f32| if cfg!(target_os = "macos") { v } else { v / raw.dpi_scale };
    let (x, y) = (to_logical(raw.x), to_logical(raw.y));
    let (lw, lh) = (to_logical(raw.w), to_logical(raw.h));
    let scale = raw.width as f32 / lw.max(1.0);
    let windows = raw
        .windows
        .iter()
        .map(|w| [to_logical(w[0]), to_logical(w[1]), to_logical(w[2]), to_logical(w[3])])
        .collect();
    Some(Frame { x, y, lw, lh, width: raw.width, height: raw.height, scale, windows, rgba: raw.rgba })
}

fn spawn_capture() -> Option<Child> {
    let frame = grab()?;
    let path = frame::temp_dir().join("frame.bin");
    frame.write(&path).ok()?;
    #[cfg(windows)]
    unsafe {
        // Let the overlay process take the foreground so it receives the keyboard.
        windows_sys::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(
            windows_sys::Win32::UI::WindowsAndMessaging::ASFW_ANY,
        );
    }
    Command::new(std::env::current_exe().ok()?).arg("--capture").arg(path).spawn().ok()
}
