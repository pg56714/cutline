# Cutline

English | [繁體中文](README.zh-TW.md)

A small, fast screenshot tool for Windows in the style of Snipaste's capture mode:
press a hotkey, the screen freezes, pick a region, annotate it, then copy, save or pin it.

Written in Rust (egui + glow). The release is a single exe (also available as an installer) with no runtime.

> Cutline is an independent project. It is not affiliated with or derived from Snipaste;
> it only follows the same general workflow.

> [!WARNING]
> **The app is not code-signed**, so the system blocks it the first time. Do this once:
> - **Windows**: "Windows protected your PC" -> **More info** -> **Run anyway**
> - **macOS**: System Settings -> Privacy & Security -> find Cutline and click **Open Anyway**, or run `xattr -cr /Applications/Cutline.app` in Terminal
> - **Linux**: run `chmod +x` on the AppImage first; the .deb installs as usual
>
> Because there is no signature, check what you downloaded: see [Verify your download](#verify-your-download).

## Download

Grab the file for your system from the [latest release](https://github.com/pg56714/cutline/releases/latest).

| System | File | Notes |
| --- | --- | --- |
| Windows | `cutline_*_x64-setup.exe` | Installer |
| Windows | `cutline_windows_x64.exe` | Portable, runs without installing |
| macOS | `Cutline_*.dmg` | Apple Silicon |
| Linux | `cutline_*_x86_64.AppImage` | `chmod +x` first |
| Linux | `cutline_*_amd64.deb` | Debian / Ubuntu: `sudo apt install ./cutline_*.deb` |

## Features

- **Global hotkey** (default `F1`) and a tray icon (`Capture` / `Start with Windows` / `Check for updates` / `Quit`).
- **Freeze, then pick**: the screen is grabbed *before* anything is drawn, so you capture exactly what you saw.
- **Hover to snap to a window**, click to select it, or drag to select any region.
- **Magnifier** with pixel coordinates and RGB under the cursor; press `C` to copy the colour as `#RRGGBB`.
- **Adjust** the selection: drag inside to move (annotations move with it), drag the 8 handles to resize.
- **Annotate**: rectangle / ellipse, arrow / line, pen, highlighter, mosaic, text (CJK works), eraser.
  Colours and sizes per tool, `Shift` constrains shapes to squares and 45° lines. Undo / redo.
  Rest the pointer on a toolbar button to see what it does and its shortcut.
- **Text** is typed in a box that grows with it and wraps at the edge of the selection: drag the box's
  right edge to set the width, drag its frame to move it, and pick a size or colour while typing to restyle it.
- **Output**: copy to clipboard, save as PNG, or **pin** the image as an always-on-top window
  (drag to move, scroll to zoom, `Ctrl+C` to copy, `Esc` / right-click / double-click to close).
- The exported image is rendered from exactly what you see, cropped to the pixel.

## Keys

| Key | Action |
| --- | --- |
| `F1` (configurable) | Start a capture |
| Click / drag | Select a window / a region |
| `Enter`, `Ctrl+C`, double-click | Copy the selection and finish |
| `Ctrl+S` | Save as PNG |
| `F3` | Pin the selection |
| `Ctrl+Z` / `Ctrl+Y` | Undo / redo |
| Right-click | Cancel the selection (or quit if none) |
| `Esc` | Quit |
| `C` (while picking) | Copy the colour under the cursor |

## Configuration

`%APPDATA%\Cutline\config.txt` is created on first run:

```
hotkey = F1
```

Any combination understood by the [`global-hotkey`](https://crates.io/crates/global-hotkey) crate works,
for example `F2` or `ctrl+shift+KeyA`. If the key is already taken by another program (Snipaste, for
instance) Cutline tells you and exits; quit the other program or pick another key.
You can also override it for one run with the `CUTLINE_HOTKEY` environment variable.

## Verify your download

Every release carries a `SHA256SUMS` file and a build attestation (GitHub's signed record that the file was built
by this repository's release workflow). Either one tells you the file is the one the workflow produced.

```powershell
# Windows: compare with the line for your file in SHA256SUMS
Get-FileHash .\cutline_windows_x64.exe -Algorithm SHA256
```

```bash
# macOS / Linux, with SHA256SUMS in the same folder
sha256sum -c --ignore-missing SHA256SUMS      # macOS: shasum -a 256 -c --ignore-missing SHA256SUMS

# Stronger: needs the GitHub CLI
gh attestation verify <file> --repo pg56714/cutline
```

`SHA256SUMS` sits next to the files on the release page, so it catches a damaged or swapped download but not a
compromised GitHub account; the attestation is the check that does. Neither replaces a code signature, so the
system warning above stays.

## Start at sign-in

Tick **Start with Windows** in the tray menu (**Start at login** on Linux and macOS) to have Cutline start
when you sign in, and untick it to stop. It is off by default. The setting is kept where the system keeps
it, not in `config.txt`:

| Platform | Entry |
| --- | --- |
| Windows | The `Cutline` value under `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run` |
| Linux | `~/.config/autostart/cutline.desktop` (`$XDG_CONFIG_HOME/autostart/cutline.desktop` if that is set). An AppImage is started through its `.AppImage` file. |
| macOS | `~/Library/LaunchAgents/app.cutline.desktop.plist` |

- The entry starts the copy of Cutline that ticked it. If you move the portable exe, or run another copy,
  the item shows unticked there; ticking it points the entry at that copy.
- Cutline reads the setting when it starts, so switching it off in the system's own settings shows as
  unticked from the next start: Task Manager → Startup apps or Settings → Apps → Startup on Windows, the
  desktop's autostart settings on Linux. Not on macOS: switching it off under System Settings → General →
  Login Items leaves the item ticked.
- Uninstalling Cutline does not remove the entry, so untick it first. A leftover entry points at a missing
  file and starts nothing.
- On Windows the registry handling is covered by unit tests, but signing in with it ticked has not been
  tested yet. The Linux and macOS entries have not been tested.

## Updates

Cutline looks for a newer [release](https://github.com/pg56714/cutline/releases) once when it starts, quietly, and again whenever you click **Check for updates** (there is no timer).
When there is one, the tray item **Check for updates** becomes **Update to vX.Y.Z**; click it to update.
Clicking it with nothing newer checks right away and says so.

| How you run it | What the update does |
| --- | --- |
| Windows installer | Downloads the new setup program and runs it |
| Windows portable `.exe`, Linux AppImage | Downloads the new file, swaps it in place and restarts Cutline |
| `.deb`, `.dmg`, built from source | Opens the release page; install it yourself |

- Downloads go through `curl`, which Windows 10+, macOS and desktop Linux already have. A download that is not a
  program (cut off, or an error page) never replaces anything. When the release has a `SHA256SUMS` file, the
  download must match it too; releases older than that have none, and are accepted as before.
- `CUTLINE_NO_UPDATE_CHECK=1` turns the check off (and the tray item with it).
- The swap needs write access to the folder the file is in.
- Tested: the version comparison, picking the right file, the download and swap (unit tests on Windows and Ubuntu),
  and that the resident process starts on Linux with an update available. Clicking the tray item has not been tried.

## Platforms

| Platform | Status |
| --- | --- |
| Windows 11 | Developed and tested, including a clean Windows 11 VM at 100% and 125% display scaling (hotkey, selection, annotations, save, pin, system clipboard, tray icon). Not yet tested above 125% scaling, with mixed-DPI monitors, or by clicking the tray menu. |
| Linux, X11 | Built and run on Ubuntu 22.04 and 24.04 (checked under a virtual X server: hotkey, single instance, selection, window snap, annotations, copy). The prebuilt release is made on Ubuntu 24.04 (glibc 2.39), so it needs a system that new; on older ones such as Ubuntu 22.04, build from source (checked to work). The tray icon needs a desktop with a status-notifier / AppIndicator host. Not tested on a real desktop session. |
| Linux, Wayland | **Not supported yet.** Cutline captures through the X server, which on Wayland would give a black or partial picture, so it shows a message instead. Screen capture on Wayland has to go through the desktop portal, which asks for permission; global hotkeys and always-on-top overlays are also restricted. Use an X11 session. `CUTLINE_FORCE_X11=1` skips the check and captures only X11 windows through XWayland. |
| macOS | Type-checks (`cargo check --target aarch64-apple-darwin`) but has **never been run**. Needs the Screen Recording permission. |

## Build

Rust is pinned in `rust-toolchain.toml` (the release workflow uses the same version), so a new stable release never
changes a build by itself. To upgrade, change `channel` there and the `dtolnay/rust-toolchain@` tag in
`.github/workflows/release.yml`.

```
cargo build --release
```

**Windows**: use the default `x86_64-pc-windows-msvc` toolchain (needs the Visual Studio Build Tools),
or the `x86_64-pc-windows-gnu` target with a MinGW-w64 toolchain on `PATH`.
The binary is `target/<target>/release/cutline.exe`.

**Linux** (Debian/Ubuntu package names):

```
sudo apt install build-essential pkg-config libgtk-3-dev libxdo-dev libayatana-appindicator3-dev
```

On Linux the screen is captured by talking to the X server directly (`src/capture_x11.rs`), so none
of the pipewire / Wayland development packages are needed. This is also what lets the build run on
Ubuntu 22.04: the pipewire bindings that xcap pulls in on Linux need a newer libpipewire than 22.04 ships.

At run time the Linux build needs `libgtk-3-0`, `libayatana-appindicator3-1`, `libxdo3` and
`libxkbcommon-x11-0` (a desktop Ubuntu has them; a minimal install may not), plus the usual OpenGL
(Mesa) libraries.

`cutline --now` takes a screenshot immediately without starting the resident tray process.

### Test harness

`cargo build --features devscript` (always on in debug builds) compiles in a scripted-input mode:
set `CUTLINE_SCRIPT` to a list of steps such as `m 300 250; d; m 900 600; u; btn Shape; s shot; btn Copy`
and the overlay replays them as synthetic input, writing screenshots to `<temp>/cutline/`. It lets the
whole flow be tested without touching the real mouse. See `src/overlay/devscript.rs` for the step list.

## How it works

One executable, three modes:

- **resident** (default): tray icon and hotkey. On the hotkey it grabs the monitor under the cursor
  and starts `cutline --capture` with the frame in a temp file.
- **`--capture`**: the full-screen overlay. It lives only for the duration of one capture.
- **`--pin`**: one pinned image. Each pin is its own small process.

## Known limitations

- Only the monitor under the cursor is captured, not the whole virtual desktop.
- Display scaling above 125% and mixed-DPI multi-monitor setups are not tested yet.
- No scrolling capture, no screen recording, no OCR.

## Contributing

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/)
(`feat(overlay): ...`, `fix: ...`, `docs: ...`). They are not just style: the release workflow reads them
to pick the next version and write the changelog.

## Releasing

From the Actions tab, run **Release** by hand. It looks at the Conventional Commits since the last tag:
`feat` bumps the minor version, `fix` and `perf` bump the patch, a `!` or a `BREAKING CHANGE:` footer is a
breaking change (the minor version while below 1.0). If nothing releasable changed it stops. Otherwise it
updates `CHANGELOG.md` and the version, tags, builds Windows (installer + portable `.exe`), macOS (universal `.dmg`) and
Linux (`.AppImage` + `.deb`), and publishes the release once every build has uploaded. The macOS and Linux builds
are unsigned and less tested than the Windows one.

`bun scripts/release.ts --dry-run` shows what the next release would contain, and `bun test scripts`
runs the script's tests.

## License

Apache License 2.0. See [LICENSE](LICENSE).
