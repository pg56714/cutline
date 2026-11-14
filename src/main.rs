// No console window in release builds; debug builds keep it for logging.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
#[cfg(target_os = "linux")]
mod capture_x11;
mod daemon;
mod dwm;
mod frame;
mod overlay;
mod pin;

fn main() {
    daemon::set_dpi_aware();
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--capture") => overlay::run(&args[1..]),
        Some("--pin") => pin::run(&args[1..]),
        // `--now` takes a screenshot immediately without the resident daemon
        // (handy for testing and for binding the tool to your own launcher).
        Some("--now") => daemon::capture_now(),
        _ => daemon::run(),
    }
}
