//! Screen capture for Linux/X11, written directly against the X protocol (x11rb, pure Rust).
//!
//! Cutline only supports X11 on Linux, so this replaces xcap there. xcap drags in the pipewire and
//! Wayland client libraries, which need a newer libpipewire than Ubuntu 22.04 ships and are of no
//! use to an X11-only tool. Windows and macOS still use xcap.

use x11rb::connection::Connection;
use x11rb::protocol::randr::ConnectionExt as _;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, ImageFormat, ImageOrder, MapState, Window};
use x11rb::rust_connection::RustConnection;

type Res<T> = Result<T, Box<dyn std::error::Error>>;

/// One monitor's pixels plus the geometry needed to place the overlay and snap to windows.
pub struct Capture {
    /// Monitor origin and size in X pixels (virtual screen space).
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    /// Pixels per logical point, as winit will see it (`WINIT_X11_SCALE_FACTOR` or `Xft.dpi`).
    pub dpi_scale: f32,
    pub rgba: Vec<u8>,
    /// Visible top-level windows `[x, y, w, h]` in X pixels, topmost first.
    pub windows: Vec<[i32; 4]>,
}

/// Wayland sessions hand out screen contents only through the desktop portal, which this tool does
/// not use yet. Capturing through XWayland would silently give a black or partial picture, so say so.
pub fn is_wayland() -> bool {
    if std::env::var_os("CUTLINE_FORCE_X11").is_some() {
        return false;
    }
    std::env::var("XDG_SESSION_TYPE").map(|v| v.eq_ignore_ascii_case("wayland")).unwrap_or(false)
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

pub fn grab() -> Res<Capture> {
    let (conn, screen_num) = RustConnection::connect(None)?;
    let screen = conn.setup().roots[screen_num].clone();
    let root = screen.root;

    // The monitor under the cursor; the whole screen if RandR cannot tell.
    let ptr = conn.query_pointer(root)?.reply()?;
    let (px, py) = (ptr.root_x as i32, ptr.root_y as i32);
    let mut mon = (0i32, 0i32, screen.width_in_pixels as u32, screen.height_in_pixels as u32);
    if let Ok(cookie) = conn.randr_get_monitors(root, true) {
        if let Ok(rep) = cookie.reply() {
            let under_cursor = |m: &&x11rb::protocol::randr::MonitorInfo| {
                px >= m.x as i32 && px < m.x as i32 + m.width as i32 && py >= m.y as i32 && py < m.y as i32 + m.height as i32
            };
            let pick = rep
                .monitors
                .iter()
                .find(under_cursor)
                .or_else(|| rep.monitors.iter().find(|m| m.primary))
                .or_else(|| rep.monitors.first());
            if let Some(m) = pick {
                mon = (m.x as i32, m.y as i32, m.width as u32, m.height as u32);
            }
        }
    }

    let img = conn
        .get_image(ImageFormat::Z_PIXMAP, root, mon.0 as i16, mon.1 as i16, mon.2 as u16, mon.3 as u16, !0)?
        .reply()?;
    let fmt = conn
        .setup()
        .pixmap_formats
        .iter()
        .find(|f| f.depth == img.depth)
        .ok_or("the X server reported no pixmap format for the root window")?;
    if fmt.bits_per_pixel != 32 {
        return Err(format!("unsupported X pixel format ({} bits per pixel)", fmt.bits_per_pixel).into());
    }
    let n = (mon.2 * mon.3) as usize;
    if img.data.len() < n * 4 {
        return Err("the X server returned a short image".into());
    }
    // 32 bpp, standard masks: bytes are B G R X on little-endian servers, X R G B on big-endian ones.
    let lsb = conn.setup().image_byte_order == ImageOrder::LSB_FIRST;
    let mut rgba = Vec::with_capacity(n * 4);
    for p in img.data.chunks_exact(4).take(n) {
        if lsb {
            rgba.extend_from_slice(&[p[2], p[1], p[0], 255]);
        } else {
            rgba.extend_from_slice(&[p[1], p[2], p[3], 255]);
        }
    }

    Ok(Capture {
        x: mon.0,
        y: mon.1,
        w: mon.2,
        h: mon.3,
        dpi_scale: dpi_scale(&conn, root),
        rgba,
        windows: windows(&conn, root),
    })
}

/// Same idea as winit: an explicit override first, then the `Xft.dpi` X resource.
fn dpi_scale(conn: &RustConnection, root: Window) -> f32 {
    if let Some(f) = std::env::var("WINIT_X11_SCALE_FACTOR").ok().and_then(|v| v.parse::<f32>().ok()) {
        if f > 0.0 {
            return f;
        }
    }
    let text = conn
        .get_property(false, root, AtomEnum::RESOURCE_MANAGER, AtomEnum::STRING, 0, 1 << 16)
        .ok()
        .and_then(|c| c.reply().ok())
        .map(|r| String::from_utf8_lossy(&r.value).into_owned())
        .unwrap_or_default();
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("Xft.dpi:") {
            if let Ok(dpi) = v.trim().parse::<f32>() {
                if dpi >= 48.0 {
                    return dpi / 96.0;
                }
            }
        }
    }
    1.0
}

fn atom(conn: &RustConnection, name: &[u8]) -> Option<u32> {
    conn.intern_atom(false, name).ok()?.reply().ok().map(|r| r.atom)
}

fn title(conn: &RustConnection, w: Window, net_wm_name: Option<u32>, utf8: Option<u32>) -> String {
    if let (Some(n), Some(u)) = (net_wm_name, utf8) {
        if let Some(r) = conn.get_property(false, w, n, u, 0, 256).ok().and_then(|c| c.reply().ok()) {
            if !r.value.is_empty() {
                return String::from_utf8_lossy(&r.value).into_owned();
            }
        }
    }
    conn.get_property(false, w, AtomEnum::WM_NAME, AtomEnum::STRING, 0, 256)
        .ok()
        .and_then(|c| c.reply().ok())
        .map(|r| String::from_utf8_lossy(&r.value).into_owned())
        .unwrap_or_default()
}

/// Visible, titled top-level windows, topmost first.
fn windows(conn: &RustConnection, root: Window) -> Vec<[i32; 4]> {
    let net_list = atom(conn, b"_NET_CLIENT_LIST_STACKING");
    let net_name = atom(conn, b"_NET_WM_NAME");
    let utf8 = atom(conn, b"UTF8_STRING");
    let net_state = atom(conn, b"_NET_WM_STATE");
    let hidden = atom(conn, b"_NET_WM_STATE_HIDDEN");

    // A window manager that speaks EWMH lists client windows bottom to top.
    let mut ids: Vec<Window> = Vec::new();
    if let Some(a) = net_list {
        if let Some(r) = conn.get_property(false, root, a, AtomEnum::WINDOW, 0, 8192).ok().and_then(|c| c.reply().ok()) {
            if let Some(it) = r.value32() {
                ids = it.collect();
            }
        }
    }
    // Without one (a bare X server, or a minimal window manager) fall back to the root's children.
    if ids.is_empty() {
        if let Some(r) = conn.query_tree(root).ok().and_then(|c| c.reply().ok()) {
            ids = r.children;
        }
    }
    ids.reverse();

    let mut out = Vec::new();
    for w in ids {
        let Some(attrs) = conn.get_window_attributes(w).ok().and_then(|c| c.reply().ok()) else { continue };
        if attrs.map_state != MapState::VIEWABLE {
            continue;
        }
        if let (Some(s), Some(h)) = (net_state, hidden) {
            let is_hidden = conn
                .get_property(false, w, s, AtomEnum::ATOM, 0, 64)
                .ok()
                .and_then(|c| c.reply().ok())
                .and_then(|r| r.value32().map(|mut it| it.any(|a| a == h)))
                .unwrap_or(false);
            if is_hidden {
                continue;
            }
        }
        let Some(geo) = conn.get_geometry(w).ok().and_then(|c| c.reply().ok()) else { continue };
        if geo.width < 24 || geo.height < 24 {
            continue;
        }
        if title(conn, w, net_name, utf8).is_empty() {
            continue;
        }
        // Geometry is relative to the parent, which is a decoration frame under most window managers.
        let Some(pos) = conn.translate_coordinates(w, root, 0, 0).ok().and_then(|c| c.reply().ok()) else { continue };
        out.push([pos.dst_x as i32, pos.dst_y as i32, geo.width as i32, geo.height as i32]);
    }
    out
}
