//! Windows animates a top-level window fading in and out when it is shown or hidden. For the
//! overlay and pin windows that fade reads as a flash (the dimmed screen slowly turning bright), so
//! turn the transitions off: the window then appears and disappears in a single frame.

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
