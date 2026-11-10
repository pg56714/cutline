# Cutline

A small, fast screenshot tool for Windows in the style of Snipaste's capture mode:
press a hotkey, the screen freezes, pick a region, annotate it, then copy, save or pin it.

Written in Rust (egui + glow). The release binary is a single exe with no installer and no runtime.

> Cutline is an independent project. It is not affiliated with or derived from Snipaste;
> it only follows the same general workflow.

## Features

- **Global hotkey** (default `F1`) and a tray icon (`Capture` / `Quit`).
- **Freeze, then pick**: the screen is grabbed *before* anything is drawn, so you capture exactly what you saw.
- **Hover to snap to a window**, click to select it, or drag to select any region.
- **Magnifier** with pixel coordinates and RGB under the cursor; press `C` to copy the colour as `#RRGGBB`.
- **Adjust** the selection: drag inside to move (annotations move with it), drag the 8 handles to resize.
- **Annotate**: rectangle / ellipse, arrow / line, pen, highlighter, mosaic, text (CJK works), eraser.
  Colours and sizes per tool, `Shift` constrains shapes to squares and 45° lines. Undo / redo.
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

## Platforms

| Platform | Status |
| --- | --- |
| Windows 11 | Developed and tested here (hotkey, selection, annotations, save, pin, copy-to-file). Not yet verified on a clean machine, at display scaling above 100%, or the tray menu / system clipboard path. |
| Linux, X11 | Builds and runs (checked under Xvfb: hotkey, single instance, selection, window snap, annotations, copy). The tray icon needs a desktop with a status-notifier / AppIndicator host. |
| Linux, Wayland | **Not supported yet.** Screen capture goes through the desktop portal, which asks for permission and cannot be driven the way this tool needs; global hotkeys and always-on-top overlays are also restricted. Use an X11 session. |
| macOS | Type-checks (`cargo check --target aarch64-apple-darwin`) but has **never been run**. Needs the Screen Recording permission. |

## Build

```
cargo build --release
```

**Windows**: use the default `x86_64-pc-windows-msvc` toolchain (needs the Visual Studio Build Tools),
or the `x86_64-pc-windows-gnu` target with a MinGW-w64 toolchain on `PATH`.
The binary is `target/<target>/release/cutline.exe`.

**Linux** (Debian/Ubuntu package names):

```
sudo apt install build-essential pkg-config libgtk-3-dev libxdo-dev libayatana-appindicator3-dev \
  libpipewire-0.3-dev libspa-0.2-dev libclang-dev libgbm-dev libdrm-dev libxcb1-dev \
  libxcb-randr0-dev libxcb-shm0-dev libxcb-xfixes0-dev libxkbcommon-dev libdbus-1-dev \
  libegl1-mesa-dev libgl1-mesa-dev libwayland-dev
```

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
- Display scaling other than 100% and mixed-DPI multi-monitor setups are not tested yet.
- A pinned image can sit about 1 px lower than the original on Windows.
- No scrolling capture, no screen recording, no OCR.

## License

Apache License 2.0. See [LICENSE](LICENSE).
