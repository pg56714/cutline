//! Start the resident process when the user signs in. Off until it is ticked in the tray menu.
//!
//! The setting is kept where the system keeps it, not in config.txt, so switching it off in the
//! system's own settings unticks the tray item too:
//!
//! - Windows: a `Cutline` value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
//! - Linux: an XDG autostart entry, `~/.config/autostart/cutline.desktop`.
//! - macOS: a LaunchAgent, `~/Library/LaunchAgents/app.cutline.desktop.plist`.
//!
//! The entry names one copy of Cutline, and the item is ticked only while it names the copy that is
//! running: after the portable exe has moved, or with a second copy around, it shows unticked, and
//! ticking it points the entry here.

use std::path::PathBuf;

/// The tray menu item.
pub const LABEL: &str = if cfg!(windows) { "Start with Windows" } else { "Start at login" };

/// Whether signing in starts this copy of Cutline.
pub fn is_enabled() -> bool {
    launch_path().is_some_and(|exe| system().is_enabled(&exe))
}

/// Make signing in start this copy of Cutline (`on`), or no Cutline at all.
pub fn set_enabled(on: bool) -> Result<(), String> {
    let exe = launch_path().ok_or("could not find the path of the Cutline executable")?;
    // An app opened where it was downloaded runs from a temporary copy until it is moved.
    #[cfg(target_os = "macos")]
    if on && exe.to_string_lossy().contains("/AppTranslocation/") {
        return Err("macOS is running Cutline from a temporary copy. Move Cutline to the Applications \
                    folder with the Finder, open it from there and try again."
            .into());
    }
    system().set_enabled(&exe, on)
}

/// What the system should start: this executable, or on Linux the AppImage it runs from.
fn launch_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    #[cfg(target_os = "linux")]
    let exe = appimage_or(exe, std::env::var_os("APPIMAGE"), std::env::var_os("APPDIR"));
    Some(exe)
}

/// An AppImage runs from a mount point that changes on every run, so the entry has to name the
/// `.AppImage` file itself. Its runtime says which in `$APPIMAGE`, and where it is mounted in
/// `$APPDIR`. Everything started from an AppImage inherits both, so they only count while this
/// executable is inside that mount.
#[cfg(any(test, target_os = "linux"))]
fn appimage_or(exe: PathBuf, appimage: Option<std::ffi::OsString>, appdir: Option<std::ffi::OsString>) -> PathBuf {
    match (appimage, appdir) {
        (Some(image), Some(dir)) if !image.is_empty() && !dir.is_empty() && exe.starts_with(&dir) => image.into(),
        _ => exe,
    }
}

#[cfg(windows)]
fn system() -> registry::Keys<'static> {
    registry::SYSTEM
}

#[cfg(target_os = "linux")]
fn system() -> file::EntryFile {
    xdg::entry()
}

#[cfg(target_os = "macos")]
fn system() -> file::EntryFile {
    launchd::entry()
}

/// Windows: a value in the per-user Run key, which Explorer starts at sign-in. Needs no admin rights.
#[cfg(windows)]
mod registry {
    use std::path::Path;
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, ERROR_SUCCESS};
    use windows_sys::Win32::System::Registry::{
        HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_BINARY, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
    };

    /// The value's name, in both keys.
    const NAME: &str = "Cutline";

    /// Two subkeys of HKEY_CURRENT_USER.
    pub struct Keys<'a> {
        /// The command lines Explorer runs at sign-in.
        pub run: &'a str,
        /// Where Task Manager's Startup apps page (and Settings > Apps > Startup) switch an entry
        /// off. They leave its Run value alone.
        pub approved: &'a str,
    }

    pub const SYSTEM: Keys<'static> = Keys {
        run: r"Software\Microsoft\Windows\CurrentVersion\Run",
        approved: r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run",
    };

    impl Keys<'_> {
        pub fn is_enabled(&self, exe: &Path) -> bool {
            get_string(self.run, NAME).is_some_and(|cmd| starts(&cmd, exe))
                && !switched_off(get(self.approved, NAME, RRF_RT_REG_BINARY))
        }

        pub fn set_enabled(&self, exe: &Path, on: bool) -> Result<(), String> {
            if on {
                // Quoted: the path may have spaces (`C:\Program Files\...`).
                set_string(self.run, NAME, &format!("\"{}\"", exe.display()))?;
            } else {
                delete(self.run, NAME)?;
            }
            // Drop a Task Manager switch-off too, or it would outlast ticking the item here.
            delete(self.approved, NAME)
        }
    }

    /// Whether the Run command line `cmd` starts `exe`. Paths are case-insensitive on Windows.
    fn starts(cmd: &str, exe: &Path) -> bool {
        let cmd = cmd.trim();
        let program = match cmd.strip_prefix('"') {
            Some(rest) => rest.split('"').next().unwrap_or_default(),
            None => cmd.split_whitespace().next().unwrap_or_default(),
        };
        !program.is_empty() && program.to_lowercase() == exe.to_string_lossy().to_lowercase()
    }

    /// StartupApproved data is a flags word, then the time the entry was switched off. Windows
    /// writes 2 for on and 3 for off (some versions 6 and 7), so an odd first byte is off. No value
    /// is on.
    fn switched_off(data: Option<Vec<u8>>) -> bool {
        data.and_then(|d| d.first().copied()).is_some_and(|flags| flags & 1 == 1)
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain([0]).collect()
    }

    /// A value's data, if it exists and has the type `kind` (an `RRF_RT_*` flag).
    fn get(subkey: &str, name: &str, kind: u32) -> Option<Vec<u8>> {
        let (subkey, name) = (wide(subkey), wide(name));
        let mut len = 0u32;
        // First the size, then the data.
        let err = unsafe {
            RegGetValueW(HKEY_CURRENT_USER, subkey.as_ptr(), name.as_ptr(), kind, null_mut(), null_mut(), &mut len)
        };
        if err != ERROR_SUCCESS {
            return None;
        }
        let mut data = vec![0u8; len as usize];
        let err = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                name.as_ptr(),
                kind,
                null_mut(),
                data.as_mut_ptr().cast(),
                &mut len,
            )
        };
        if err != ERROR_SUCCESS {
            return None;
        }
        data.truncate(len as usize);
        Some(data)
    }

    fn get_string(subkey: &str, name: &str) -> Option<String> {
        let data = get(subkey, name, RRF_RT_REG_SZ)?;
        let units: Vec<u16> =
            data.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&u| u != 0).collect();
        Some(String::from_utf16_lossy(&units))
    }

    fn set_string(subkey: &str, name: &str, value: &str) -> Result<(), String> {
        let (key, value_name, data) = (wide(subkey), wide(name), wide(value));
        let err = unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                value_name.as_ptr(),
                REG_SZ,
                data.as_ptr().cast(),
                (data.len() * 2) as u32,
            )
        };
        if err == ERROR_SUCCESS { Ok(()) } else { Err(failed("write", subkey, name, err)) }
    }

    /// Deleting a value that is not there is fine.
    fn delete(subkey: &str, name: &str) -> Result<(), String> {
        let (key, value_name) = (wide(subkey), wide(name));
        match unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), value_name.as_ptr()) } {
            ERROR_SUCCESS | ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => Ok(()),
            err => Err(failed("remove", subkey, name, err)),
        }
    }

    fn failed(action: &str, subkey: &str, name: &str, err: u32) -> String {
        let err = std::io::Error::from_raw_os_error(err as i32);
        format!("could not {action} {name} in HKEY_CURRENT_USER\\{subkey}: {err}")
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use windows_sys::Win32::System::Registry::{REG_BINARY, RegDeleteTreeW};

        /// Scratch keys under HKCU\Software in place of the real ones, one set per test (tests run
        /// in parallel), deleted afterwards.
        struct Scratch {
            root: String,
            run: String,
            approved: String,
        }

        impl Scratch {
            fn new(test: &str) -> Self {
                let root = format!(r"Software\cutline-test-{}-{test}", std::process::id());
                Scratch { run: format!(r"{root}\Run"), approved: format!(r"{root}\Approved"), root }
            }

            fn keys(&self) -> Keys<'_> {
                Keys { run: &self.run, approved: &self.approved }
            }
        }

        impl Drop for Scratch {
            fn drop(&mut self) {
                let root = wide(&self.root);
                unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, root.as_ptr()) };
            }
        }

        fn set_binary(subkey: &str, data: &[u8]) {
            let (key, value_name) = (wide(subkey), wide(NAME));
            let err = unsafe {
                RegSetKeyValueW(
                    HKEY_CURRENT_USER,
                    key.as_ptr(),
                    value_name.as_ptr(),
                    REG_BINARY,
                    data.as_ptr().cast(),
                    data.len() as u32,
                )
            };
            assert_eq!(err, ERROR_SUCCESS);
        }

        const EXE: &str = r"C:\Program Files\Cutline\cutline.exe";

        #[test]
        fn turn_on_and_off() {
            let scratch = Scratch::new("on-off");
            let keys = scratch.keys();
            let exe = Path::new(EXE);
            assert!(!keys.is_enabled(exe));

            keys.set_enabled(exe, true).unwrap();
            assert_eq!(get_string(keys.run, NAME).as_deref(), Some(r#""C:\Program Files\Cutline\cutline.exe""#));
            assert!(keys.is_enabled(exe));
            // It does not start another copy of Cutline.
            assert!(!keys.is_enabled(Path::new(r"D:\Tools\cutline.exe")));

            keys.set_enabled(exe, false).unwrap();
            assert_eq!(get_string(keys.run, NAME), None);
            assert!(!keys.is_enabled(exe));
            // Turning off what is off already is fine.
            keys.set_enabled(exe, false).unwrap();
        }

        #[test]
        fn task_manager_switch_off() {
            let scratch = Scratch::new("task-manager");
            let keys = scratch.keys();
            let exe = Path::new(EXE);
            keys.set_enabled(exe, true).unwrap();

            // What Task Manager writes when Cutline is disabled there: 3, then a FILETIME...
            set_binary(keys.approved, &[3, 0, 0, 0, 0xe0, 0x2c, 0x1a, 0xd3, 0xaa, 0xe7, 0xd9, 0x01]);
            assert!(!keys.is_enabled(exe));
            // ...and when it is enabled again: 2, then zeros.
            set_binary(keys.approved, &[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            assert!(keys.is_enabled(exe));

            // Ticking the item clears a switch-off.
            set_binary(keys.approved, &[3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            keys.set_enabled(exe, true).unwrap();
            assert_eq!(get(keys.approved, NAME, RRF_RT_REG_BINARY), None);
            assert!(keys.is_enabled(exe));
        }

        #[test]
        fn command_lines() {
            let exe = Path::new(EXE);
            assert!(starts(r#""C:\Program Files\Cutline\cutline.exe""#, exe));
            assert!(starts(r#" "c:\program files\cutline\CUTLINE.EXE" --flag"#, exe));
            assert!(starts(r"C:\Tools\cutline.exe --flag", Path::new(r"C:\Tools\cutline.exe")));
            assert!(!starts(r#""C:\Program Files\Other\cutline.exe""#, exe));
            assert!(!starts("", exe));
            assert!(!starts(r#""""#, exe));
        }

        #[test]
        fn switch_off_flags() {
            assert!(!switched_off(None));
            assert!(!switched_off(Some(vec![])));
            assert!(!switched_off(Some(vec![2, 0, 0, 0])));
            assert!(switched_off(Some(vec![3, 0, 0, 0])));
            assert!(!switched_off(Some(vec![6, 0, 0, 0])));
            assert!(switched_off(Some(vec![7, 0, 0, 0])));
        }
    }
}

/// Linux and macOS keep it in one file. Built into the tests everywhere, so this machine checks it.
#[cfg(any(test, not(windows)))]
mod file {
    use std::path::{Path, PathBuf};

    /// One file, written to turn it on and removed to turn it off.
    pub struct EntryFile {
        /// None when `$HOME` is not set.
        pub path: Option<PathBuf>,
        /// The file's text for an executable.
        pub contents: fn(&str) -> String,
        /// Whether a file's text starts an executable, and is not switched off.
        pub starts: fn(&str, &str) -> bool,
    }

    impl EntryFile {
        pub fn is_enabled(&self, exe: &Path) -> bool {
            let (Some(path), Some(exe)) = (&self.path, exe.to_str()) else { return false };
            std::fs::read_to_string(path).is_ok_and(|text| (self.starts)(&text, exe))
        }

        pub fn set_enabled(&self, exe: &Path, on: bool) -> Result<(), String> {
            let path = self.path.as_deref().ok_or("$HOME is not set")?;
            if !on {
                return match std::fs::remove_file(path) {
                    Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                        Err(format!("could not remove {}: {e}", path.display()))
                    }
                    _ => Ok(()),
                };
            }
            let exe = exe.to_str().ok_or("the path of the Cutline executable is not valid UTF-8")?;
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
            }
            std::fs::write(path, (self.contents)(exe)).map_err(|e| format!("could not write {}: {e}", path.display()))
        }
    }
}

/// Linux: an entry in the XDG autostart folder, which the desktop session starts at login.
#[cfg(any(test, target_os = "linux"))]
mod xdg {
    use super::file::EntryFile;
    use std::path::PathBuf;

    pub fn entry() -> EntryFile {
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
        EntryFile { path: config.map(|c| c.join("autostart").join("cutline.desktop")), contents, starts }
    }

    fn contents(exe: &str) -> String {
        format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=Cutline\n\
             Comment=Screenshot tool\n\
             Exec={}\n\
             Terminal=false\n\
             X-GNOME-Autostart-enabled=true\n",
            exec(exe)
        )
    }

    /// Desktops switch an entry off without deleting it: `Hidden=true` (the autostart spec's own
    /// way) or GNOME's `X-GNOME-Autostart-enabled=false`.
    fn starts(text: &str, exe: &str) -> bool {
        let mut ours = false;
        for (key, value) in text.lines().filter_map(|line| line.split_once('=')) {
            match (key.trim(), value.trim()) {
                ("Exec", value) => ours = value == exec(exe),
                ("Hidden", "true") | ("X-GNOME-Autostart-enabled", "false") => return false,
                _ => {}
            }
        }
        ours
    }

    /// `Exec` quoting from the Desktop Entry spec: the path in double quotes, with `"`, `` ` ``, `$`
    /// and `\` behind a backslash, which has to be doubled because string escapes are undone first.
    /// `%` starts a field code, so it is doubled too.
    fn exec(exe: &str) -> String {
        let mut out = String::from('"');
        for c in exe.chars() {
            match c {
                '"' | '`' | '$' => {
                    out.push_str(r"\\");
                    out.push(c);
                }
                '\\' => out.push_str(r"\\\\"),
                '%' => out.push_str("%%"),
                _ => out.push(c),
            }
        }
        out.push('"');
        out
    }
}

/// macOS: a LaunchAgent, which launchd loads at login. Only the file is written or removed:
/// `launchctl bootout` would also stop the running Cutline (it is the agent's process), and the next
/// login reads the file anyway.
///
/// Switching it off under System Settings > General > Login Items does not show in the tray menu:
/// macOS keeps that switch to itself.
#[cfg(any(test, target_os = "macos"))]
mod launchd {
    use super::file::EntryFile;
    use std::path::PathBuf;

    /// The bundle identifier (`[package.metadata.packager]` in Cargo.toml).
    const LABEL: &str = "app.cutline.desktop";

    pub fn entry() -> EntryFile {
        let path = std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join("Library/LaunchAgents").join(format!("{LABEL}.plist")));
        EntryFile { path, contents, starts }
    }

    fn contents(exe: &str) -> String {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
             <plist version=\"1.0\">\n\
             <dict>\n\
             \t<key>Label</key>\n\
             \t<string>{LABEL}</string>\n\
             \t<key>ProgramArguments</key>\n\
             \t<array>\n\
             \t\t<string>{}</string>\n\
             \t</array>\n\
             \t<key>RunAtLoad</key>\n\
             \t<true/>\n\
             </dict>\n\
             </plist>\n",
            xml(exe)
        )
    }

    fn starts(text: &str, exe: &str) -> bool {
        text.contains(&format!("<string>{}</string>", xml(exe)))
    }

    fn xml(text: &str) -> String {
        text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
    }
}

#[cfg(test)]
mod tests {
    use super::file::EntryFile;
    use super::*;
    use std::ffi::OsString;
    use std::path::Path;

    /// A fresh folder for one test (tests run in parallel).
    fn scratch(test: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cutline-test-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn appimage() {
        let mounted = PathBuf::from("/tmp/.mount_CutlinXyz/usr/bin/cutline");
        let image = || Some(OsString::from("/home/me/Apps/Cutline.AppImage"));
        let dir = || Some(OsString::from("/tmp/.mount_CutlinXyz"));
        assert_eq!(appimage_or(mounted.clone(), image(), dir()), PathBuf::from("/home/me/Apps/Cutline.AppImage"));
        // Installed some other way, but started from an AppImage (a launcher, a terminal).
        let installed = PathBuf::from("/usr/bin/cutline");
        assert_eq!(appimage_or(installed.clone(), image(), dir()), installed);
        assert_eq!(appimage_or(installed.clone(), None, None), installed);
        assert_eq!(appimage_or(mounted.clone(), Some(OsString::new()), dir()), mounted);
    }

    #[test]
    fn xdg_entry() {
        let dir = scratch("xdg");
        let path = dir.join("autostart").join("cutline.desktop");
        let entry = EntryFile { path: Some(path.clone()), ..xdg::entry() };
        let exe = Path::new("/opt/My Apps/cutline");
        assert!(!entry.is_enabled(exe));

        entry.set_enabled(exe, true).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("[Desktop Entry]\n"), "{text}");
        assert!(text.contains("\nExec=\"/opt/My Apps/cutline\"\n"), "{text}");
        assert!(entry.is_enabled(exe));
        assert!(!entry.is_enabled(Path::new("/usr/bin/cutline")));

        // Switched off by the desktop, then ticked again.
        let gnome_off = text.replace("X-GNOME-Autostart-enabled=true", "X-GNOME-Autostart-enabled=false");
        std::fs::write(&path, gnome_off).unwrap();
        assert!(!entry.is_enabled(exe));
        std::fs::write(&path, format!("{text}Hidden=true\n")).unwrap();
        assert!(!entry.is_enabled(exe));
        entry.set_enabled(exe, true).unwrap();
        assert!(entry.is_enabled(exe));

        entry.set_enabled(exe, false).unwrap();
        assert!(!path.exists());
        entry.set_enabled(exe, false).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn xdg_exec_quoting() {
        let exe = r#"/opt/a "b" $c `d` \e 100%/cutline"#;
        let entry = xdg::entry();
        let text = (entry.contents)(exe);
        assert!(text.contains(r#"Exec="/opt/a \\"b\\" \\$c \\`d\\` \\\\e 100%%/cutline""#), "{text}");
        assert!((entry.starts)(&text, exe));
    }

    #[test]
    fn launch_agent() {
        let dir = scratch("launchd");
        let path = dir.join("LaunchAgents").join("app.cutline.desktop.plist");
        let entry = EntryFile { path: Some(path.clone()), ..launchd::entry() };
        let exe = Path::new("/Applications/Cut & Paste/Cutline.app/Contents/MacOS/cutline");
        assert!(!entry.is_enabled(exe));

        entry.set_enabled(exe, true).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("<key>Label</key>\n\t<string>app.cutline.desktop</string>"), "{text}");
        assert!(
            text.contains("<array>\n\t\t<string>/Applications/Cut &amp; Paste/Cutline.app/Contents/MacOS/cutline</string>"),
            "{text}"
        );
        assert!(text.contains("<key>RunAtLoad</key>\n\t<true/>"), "{text}");
        assert!(entry.is_enabled(exe));
        assert!(!entry.is_enabled(Path::new("/Applications/Cutline.app/Contents/MacOS/cutline")));

        entry.set_enabled(exe, false).unwrap();
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
