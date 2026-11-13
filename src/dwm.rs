//! Windows-only fixes for how the overlay and pin windows are shown.
//!
//! Windows animates a top-level window fading in and out when it is shown or hidden. For the
//! overlay and pin windows that fade reads as a flash (the dimmed screen slowly turning bright), so
//! turn the transitions off: the window then appears and disappears in a single frame.
//!
//! Windows 11 also frames every window that has a caption style, and winit keeps that style on
//! undecorated windows: a grey one-pixel border and rounded corners, and (winit's way of keeping the
//! drop shadow) a client area pushed down by one pixel. The picture then sits one pixel lower than
//! the screen it was taken from, loses its last row and its corners, and a pin has its own outline
//! covered on three sides. `own_pixels` gives the window back exactly its own pixels.

/// What `own_pixels` is setting up.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Covers a monitor: no shadow (it would spill onto the next monitor).
    Overlay,
    /// A small floating picture: keeps the drop shadow.
    Pin,
}

#[cfg(windows)]
pub fn disable_transitions() {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::Graphics::Dwm::{DWMWA_TRANSITIONS_FORCEDISABLED, DwmSetWindowAttribute};
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::WindowsAndMessaging::EnumThreadWindows;

    unsafe extern "system" fn each(hwnd: HWND, _: LPARAM) -> i32 {
        let on: i32 = 1;
        unsafe {
            DwmSetWindowAttribute(hwnd, DWMWA_TRANSITIONS_FORCEDISABLED as u32, &on as *const i32 as *const _, 4);
        }
        1
    }
    unsafe {
        EnumThreadWindows(GetCurrentThreadId(), Some(each), 0);
    }
}

#[cfg(not(windows))]
pub fn disable_transitions() {}

/// Square corners, no system border, and a client area that is the whole window.
/// Call once, before the window is shown. A process has one such window (the overlay or one pin).
#[cfg(windows)]
pub fn own_pixels(frame: &eframe::Frame, kind: Kind) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::sync::atomic::{AtomicIsize, Ordering};
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::Graphics::Dwm::{
        DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
        DwmExtendFrameIntoClientArea, DwmSetWindowAttribute,
    };
    use windows_sys::Win32::UI::Controls::MARGINS;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, GWLP_WNDPROC, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
        SetWindowLongPtrW, SetWindowPos, WM_NCCALCSIZE, WNDPROC,
    };

    /// winit's window procedure, which ours wraps.
    static WINIT_PROC: AtomicIsize = AtomicIsize::new(0);

    unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        // Answering 0 without touching the rectangle keeps the one Windows proposes: the whole
        // window. winit would move it down a pixel here.
        if msg == WM_NCCALCSIZE && wparam != 0 {
            return 0;
        }
        let prev: WNDPROC = unsafe { std::mem::transmute(WINIT_PROC.load(Ordering::Relaxed)) };
        unsafe { CallWindowProcW(prev, hwnd, msg, wparam, lparam) }
    }

    let Ok(handle) = frame.window_handle() else { return };
    let RawWindowHandle::Win32(win) = handle.as_raw() else { return };
    let hwnd = win.hwnd.get() as HWND;
    if WINIT_PROC.load(Ordering::Relaxed) != 0 {
        return;
    }
    unsafe {
        // Both attributes are Windows 11 only; older systems refuse them and have nothing to undo.
        let square: i32 = DWMWCP_DONOTROUND;
        DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE as u32, &square as *const i32 as *const _, 4);
        let none: u32 = DWMWA_COLOR_NONE;
        DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR as u32, &none as *const u32 as *const _, 4);

        let prev = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as usize as _);
        WINIT_PROC.store(prev as isize, Ordering::Relaxed);
        // Have the client area worked out again, now by `proc`.
        let flags = SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE;
        SetWindowPos(hwnd, std::ptr::null_mut(), 0, 0, 0, 0, flags);
        if kind == Kind::Pin {
            // Windows only shadows a window that has some frame. One row of it under the bottom of
            // the client area is enough, and stays hidden behind the picture.
            let frame = MARGINS { cxLeftWidth: 0, cxRightWidth: 0, cyTopHeight: 0, cyBottomHeight: 1 };
            DwmExtendFrameIntoClientArea(hwnd, &frame);
        }
    }
}

#[cfg(not(windows))]
pub fn own_pixels(_: &eframe::Frame, _: Kind) {}
