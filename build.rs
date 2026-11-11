// Embed the app icon into the Windows executable (what Explorer / the taskbar show).
//
// The build script runs on the host, so `embed-resource` would pick the host's resource compiler;
// for `*-windows-gnu` targets we call MinGW's `windres` ourselves.
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=assets/cutline.rc");
    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.contains("windows") {
        return;
    }
    if target.ends_with("-gnu") {
        let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("cutline_res.o");
        let tool = if target.starts_with("x86_64") { "x86_64-w64-mingw32-windres" } else { "windres" };
        for exe in [tool, "windres"] {
            let ok = Command::new(exe)
                .args(["--input", "assets/cutline.rc", "--output-format=coff", "--output"])
                .arg(&out)
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if ok {
                println!("cargo:rustc-link-arg-bins={}", out.display());
                return;
            }
        }
        println!("cargo:warning=windres not found: the exe will have no icon");
    } else {
        let _ = embed_resource::compile("assets/cutline.rc", embed_resource::NONE);
    }
}
