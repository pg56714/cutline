//! Looking for a newer release on GitHub and replacing the running program with it.
//!
//! Downloads go through `curl` (Windows 10+, macOS and every desktop Linux have it), so Cutline carries
//! no HTTP stack of its own. `CUTLINE_UPDATE_URL` points the check somewhere else (a test server, or a
//! `file://` JSON file); `CUTLINE_NO_UPDATE_CHECK=1` turns the check off.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

const API: &str = "https://api.github.com/repos/pg56714/cutline/releases/latest";
const PAGE: &str = "https://github.com/pg56714/cutline/releases/latest";

#[derive(Debug, Clone, PartialEq)]
pub struct Asset {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    pub tag: String,
    pub page: String,
    pub assets: Vec<Asset>,
}

impl Release {
    pub fn version(&self) -> &str {
        self.tag.trim_start_matches('v')
    }
}

/// How this copy was installed, which decides how it can be replaced.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// A Linux AppImage: the file itself is swapped.
    AppImage(PathBuf),
    /// A Windows install under Program Files or AppData\Programs: the setup program does it.
    Installer,
    /// A loose Windows exe: swapped like the AppImage.
    Portable(PathBuf),
    /// .deb, .dmg, a build from source: the release page is opened.
    Manual,
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    /// Start this program again and quit.
    Restart(PathBuf),
    /// The installer is running; quit so it can replace the files.
    Quit,
    /// Show this page to the user.
    OpenPage(String),
}

/// "1.2.3" or "v1.2.3" (a "-beta" tail is ignored) as numbers.
fn numbers(v: &str) -> Option<Vec<u64>> {
    let core = v.trim().trim_start_matches('v').split(['-', '+']).next()?;
    let n: Option<Vec<u64>> = core.split('.').map(|p| p.parse().ok()).collect();
    n.filter(|n| !n.is_empty())
}

/// Whether release tag `tag` is newer than `current`. Anything unreadable is not newer.
pub fn is_newer(current: &str, tag: &str) -> bool {
    match (numbers(current), numbers(tag)) {
        (Some(mut a), Some(mut b)) => {
            let n = a.len().max(b.len());
            a.resize(n, 0);
            b.resize(n, 0);
            b > a
        }
        _ => false,
    }
}

pub fn parse_release(json: &str) -> Result<Release, String> {
    let v: Value =
        serde_json::from_str(json).map_err(|e| format!("unreadable release data: {e}"))?;
    let tag = v["tag_name"]
        .as_str()
        .ok_or("release data has no tag_name")?
        .to_string();
    let page = v["html_url"].as_str().unwrap_or(PAGE).to_string();
    let assets = v["assets"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| {
                    Some(Asset {
                        name: x["name"].as_str()?.to_string(),
                        url: x["browser_download_url"].as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Release { tag, page, assets })
}

fn curl() -> Command {
    let mut c = Command::new("curl");
    c.args([
        "-fsSL",
        "-H",
        "User-Agent: cutline",
        "-H",
        "Accept: application/vnd.github+json",
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    c
}

fn fetch_text(url: &str) -> Result<String, String> {
    let o = curl()
        .args(["--max-time", "30", url])
        .output()
        .map_err(|e| format!("curl not found: {e}"))?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
    }
}

fn download(url: &str, to: &Path) -> Result<(), String> {
    let o = curl()
        .args(["--max-time", "900", "-o"])
        .arg(to)
        .arg(url)
        .output()
        .map_err(|e| format!("curl not found: {e}"))?;
    if o.status.success() {
        Ok(())
    } else {
        let _ = std::fs::remove_file(to);
        Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
    }
}

/// The latest release when it is newer than `current`.
pub fn check(current: &str) -> Result<Option<Release>, String> {
    let url = std::env::var("CUTLINE_UPDATE_URL").unwrap_or_else(|_| API.to_string());
    let rel = parse_release(&fetch_text(&url)?)?;
    Ok(is_newer(current, &rel.tag).then_some(rel))
}

/// False when `CUTLINE_NO_UPDATE_CHECK=1`.
pub fn enabled() -> bool {
    !std::env::var("CUTLINE_NO_UPDATE_CHECK").is_ok_and(|v| v == "1")
}

/// The kind of the program that is running now.
pub fn kind() -> Kind {
    let exe = std::env::current_exe().unwrap_or_default();
    if cfg!(target_os = "linux") {
        return match std::env::var("APPIMAGE") {
            Ok(p) if !p.is_empty() => Kind::AppImage(PathBuf::from(p)),
            _ => Kind::Manual,
        };
    }
    if cfg!(windows) {
        let s = exe.display().to_string().to_lowercase().replace('/', "\\");
        // A build from source lives under `\target\`; replacing it would be wrong.
        if s.contains("\\target\\") {
            return Kind::Manual;
        }
        // The NSIS installer puts an uninstaller next to the exe.
        let installed = s.contains("\\program files")
            || s.contains("\\appdata\\local\\programs\\")
            || exe.with_file_name("uninstall.exe").exists();
        return if installed {
            Kind::Installer
        } else {
            Kind::Portable(exe)
        };
    }
    Kind::Manual
}

/// The release file that fits `kind`.
pub fn asset_for<'a>(rel: &'a Release, kind: &Kind) -> Option<&'a Asset> {
    let find = |f: &dyn Fn(&str) -> bool| rel.assets.iter().find(|a| f(&a.name.to_lowercase()));
    match kind {
        Kind::AppImage(_) => find(&|n| {
            n.ends_with(".appimage")
                && (n.contains("x86_64") || n.contains("amd64") || !cfg!(target_arch = "aarch64"))
        }),
        Kind::Installer => find(&|n| n.ends_with("-setup.exe")),
        Kind::Portable(_) => find(&|n| n.ends_with(".exe") && !n.contains("setup")),
        Kind::Manual => None,
    }
}

/// The checksum of `name` in the text of a `SHA256SUMS` file (`<hex>  <name>` per line).
fn expected_sum(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|l| {
        let (hash, file) = l.trim().split_once(char::is_whitespace)?;
        (file.trim_start().trim_start_matches('*') == name).then(|| hash.to_ascii_lowercase())
    })
}

fn sha256_hex(p: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut f = std::fs::File::open(p).map_err(|e| e.to_string())?;
    let mut h = Sha256::new();
    std::io::copy(&mut f, &mut h).map_err(|e| e.to_string())?;
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Checks the downloaded `file` against the release's `SHA256SUMS`. A release without that file
/// (made before it existed) is accepted; one that has it but does not list this file, or lists a
/// different checksum, is refused.
fn verify(rel: &Release, asset: &Asset, file: &Path) -> Result<(), String> {
    let Some(sums) = rel.assets.iter().find(|a| a.name == "SHA256SUMS") else {
        return Ok(());
    };
    let text = fetch_text(&sums.url).map_err(|e| format!("could not fetch SHA256SUMS: {e}"))?;
    let want = expected_sum(&text, &asset.name)
        .ok_or_else(|| format!("SHA256SUMS does not list {}", asset.name))?;
    if sha256_hex(file)? != want {
        return Err("the download does not match SHA256SUMS (corrupted or tampered with)".into());
    }
    Ok(())
}

/// Downloads the release file for this copy and puts it in place. A download that is not a program
/// (an error page, a cut-off file) never replaces anything.
pub fn apply(rel: &Release) -> Result<Outcome, String> {
    apply_to(rel, &kind())
}

pub fn apply_to(rel: &Release, kind: &Kind) -> Result<Outcome, String> {
    let page = || {
        Outcome::OpenPage(if rel.page.is_empty() {
            PAGE.to_string()
        } else {
            rel.page.clone()
        })
    };
    let Some(asset) = asset_for(rel, kind) else {
        return Ok(page());
    };
    match kind {
        Kind::AppImage(target) | Kind::Portable(target) => {
            let name = target
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            // Next to the target, so the final rename stays on one disk.
            let new = target.with_file_name(format!("{name}.new"));
            download(&asset.url, &new).map_err(|e| format!("download failed: {e}"))?;
            if let Err(e) = check_program(&new).and_then(|_| verify(rel, asset, &new)) {
                let _ = std::fs::remove_file(&new);
                return Err(e);
            }
            replace(&new, target)?;
            Ok(Outcome::Restart(target.clone()))
        }
        Kind::Installer => {
            let to = std::env::temp_dir().join(&asset.name);
            download(&asset.url, &to).map_err(|e| format!("download failed: {e}"))?;
            if let Err(e) = check_program(&to).and_then(|_| verify(rel, asset, &to)) {
                let _ = std::fs::remove_file(&to);
                return Err(e);
            }
            Command::new(&to)
                .spawn()
                .map_err(|e| format!("could not start the installer: {e}"))?;
            Ok(Outcome::Quit)
        }
        Kind::Manual => Ok(page()),
    }
}

/// At least a megabyte, and starting like an executable (ELF, or MZ on Windows).
fn check_program(p: &Path) -> Result<(), String> {
    use std::io::Read;
    let mut head = [0u8; 4];
    let mut f = std::fs::File::open(p).map_err(|e| e.to_string())?;
    let len = f.metadata().map_err(|e| e.to_string())?.len();
    f.read_exact(&mut head)
        .map_err(|_| "the download is incomplete".to_string())?;
    if len < 1_000_000 || !(head == *b"\x7fELF" || head[..2] == *b"MZ") {
        return Err("the download is not a program (interrupted, or an error page)".into());
    }
    Ok(())
}

/// Puts `new` where `target` is. A running program can be renamed on Windows and Linux but not
/// overwritten, so the old file moves aside first and is put back if the move fails.
fn replace(new: &Path, target: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(new, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let old = target.with_file_name(format!("{name}.old"));
    let _ = std::fs::remove_file(&old);
    std::fs::rename(target, &old).map_err(|e| format!("could not move the old file aside: {e}"))?;
    if let Err(e) = std::fs::rename(new, target) {
        let _ = std::fs::rename(&old, target);
        let _ = std::fs::remove_file(new);
        return Err(format!("could not put the new file in place: {e}"));
    }
    Ok(())
}

/// Removes what a finished update left next to the program (the old file).
pub fn tidy() {
    if let Kind::AppImage(p) | Kind::Portable(p) = kind() {
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let _ = std::fs::remove_file(p.with_file_name(format!("{name}.old")));
    }
}

/// Starts `program` again, detached. The new process waits for this one's single-instance lock.
pub fn restart(program: &Path) -> Result<(), String> {
    let mut c = Command::new(program);
    c.env("CUTLINE_RESTARTED", "1");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0000_0008 | 0x0800_0000); // DETACHED_PROCESS | CREATE_NO_WINDOW
    }
    c.spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Opens `url` in the default browser.
pub fn open_url(url: &str) {
    #[cfg(windows)]
    let mut c = {
        use std::os::windows::process::CommandExt;
        let mut c = Command::new("rundll32");
        c.args(["url.dll,FileProtocolHandler", url])
            .creation_flags(0x0800_0000);
        c
    };
    #[cfg(target_os = "macos")]
    let mut c = {
        let mut c = Command::new("open");
        c.arg(url);
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut c = {
        let mut c = Command::new("xdg-open");
        c.arg(url);
        c
    };
    let _ = c.spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_by_number() {
        assert!(is_newer("0.4.1", "v0.4.2"));
        assert!(is_newer("0.4.9", "0.5.0"));
        assert!(is_newer("0.4.1", "v0.10.0"), "not as text");
        assert!(is_newer("0.4", "0.4.1"));
        assert!(!is_newer("0.4.1", "v0.4.1"));
        assert!(!is_newer("0.5.0", "v0.4.9"));
        assert!(!is_newer("0.4.1", "nightly"), "unreadable is not newer");
        assert!(!is_newer("0.4.1", "v0.4.1-beta"));
    }

    fn release() -> Release {
        let json = r#"{"tag_name":"v9.9.9","html_url":"https://example.test/r","assets":[
            {"name":"Cutline_9.9.9_aarch64.dmg","browser_download_url":"https://example.test/a.dmg"},
            {"name":"cutline_9.9.9_amd64.deb","browser_download_url":"https://example.test/a.deb"},
            {"name":"cutline_9.9.9_x64-setup.exe","browser_download_url":"https://example.test/setup.exe"},
            {"name":"cutline_9.9.9_x86_64.AppImage","browser_download_url":"https://example.test/a.AppImage"},
            {"name":"cutline_windows_x64.exe","browser_download_url":"https://example.test/p.exe"}]}"#;
        parse_release(json).unwrap()
    }

    #[test]
    fn parses_a_release() {
        let r = release();
        assert_eq!(r.version(), "9.9.9");
        assert_eq!(r.page, "https://example.test/r");
        assert_eq!(r.assets.len(), 5);
        assert!(parse_release("not json").is_err());
        assert!(parse_release("{}").is_err());
    }

    #[test]
    fn picks_the_file_for_each_kind() {
        let r = release();
        let name = |k: &Kind| asset_for(&r, k).map(|a| a.name.clone());
        assert_eq!(
            name(&Kind::AppImage("x".into())).as_deref(),
            Some("cutline_9.9.9_x86_64.AppImage")
        );
        assert_eq!(
            name(&Kind::Installer).as_deref(),
            Some("cutline_9.9.9_x64-setup.exe")
        );
        assert_eq!(
            name(&Kind::Portable("x".into())).as_deref(),
            Some("cutline_windows_x64.exe")
        );
        assert_eq!(name(&Kind::Manual), None);
    }

    #[test]
    fn manual_installs_open_the_release_page() {
        assert_eq!(
            apply_to(&release(), &Kind::Manual),
            Ok(Outcome::OpenPage("https://example.test/r".into()))
        );
    }

    #[test]
    fn a_non_program_is_refused() {
        let dir = std::env::temp_dir().join(format!("cutline-update-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("page.html");
        std::fs::write(&f, "<html>Not Found</html>").unwrap();
        assert!(check_program(&f).is_err());
        let mut big = b"MZ".to_vec();
        big.resize(1_100_000, 0);
        std::fs::write(&f, big).unwrap();
        assert!(check_program(&f).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_downloads_and_swaps_a_portable_copy() {
        let dir = std::env::temp_dir().join(format!("cutline-apply-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (target, served) = (dir.join("cutline.exe"), dir.join("served.exe"));
        std::fs::write(&target, "old").unwrap();
        let mut prog = b"MZ".to_vec();
        prog.resize(1_100_000, 7);
        std::fs::write(&served, &prog).unwrap();
        // curl reads file:// URLs, so no server is needed.
        let url = format!(
            "file:///{}",
            served
                .display()
                .to_string()
                .replace('\\', "/")
                .trim_start_matches('/')
        );
        let rel = Release {
            tag: "v9.9.9".into(),
            page: String::new(),
            assets: vec![Asset {
                name: "cutline_windows_x64.exe".into(),
                url,
            }],
        };
        let out = apply_to(&rel, &Kind::Portable(target.clone())).unwrap();
        assert_eq!(out, Outcome::Restart(target.clone()));
        assert_eq!(std::fs::read(&target).unwrap(), prog);
        assert!(!dir.join("cutline.exe.new").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reads_a_checksum_list() {
        let sums = "ABC123  cutline_windows_x64.exe
0f0f *other.AppImage
";
        assert_eq!(
            expected_sum(sums, "cutline_windows_x64.exe").as_deref(),
            Some("abc123")
        );
        assert_eq!(
            expected_sum(sums, "other.AppImage").as_deref(),
            Some("0f0f")
        );
        assert_eq!(expected_sum(sums, "missing.exe"), None);
    }

    #[test]
    fn a_checksum_mismatch_blocks_the_swap() {
        let dir = std::env::temp_dir().join(format!("cutline-sums-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (target, served, sums) = (
            dir.join("cutline.exe"),
            dir.join("served.exe"),
            dir.join("SHA256SUMS"),
        );
        std::fs::write(&target, "old").unwrap();
        let mut prog = b"MZ".to_vec();
        prog.resize(1_100_000, 7);
        std::fs::write(&served, &prog).unwrap();
        let url = |p: &Path| {
            format!(
                "file:///{}",
                p.display()
                    .to_string()
                    .replace('\\', "/")
                    .trim_start_matches('/')
            )
        };
        let rel = |sums_url: String| Release {
            tag: "v9.9.9".into(),
            page: String::new(),
            assets: vec![
                Asset {
                    name: "cutline_windows_x64.exe".into(),
                    url: url(&served),
                },
                Asset {
                    name: "SHA256SUMS".into(),
                    url: sums_url,
                },
            ],
        };
        // Wrong checksum: refused, and the old file is untouched.
        std::fs::write(
            &sums,
            format!(
                "{}  cutline_windows_x64.exe
",
                "0".repeat(64)
            ),
        )
        .unwrap();
        let err = apply_to(&rel(url(&sums)), &Kind::Portable(target.clone())).unwrap_err();
        assert!(err.contains("SHA256SUMS"), "{err}");
        assert_eq!(std::fs::read(&target).unwrap(), b"old");
        assert!(!dir.join("cutline.exe.new").exists());
        // Right checksum: swapped.
        std::fs::write(
            &sums,
            format!(
                "{}  cutline_windows_x64.exe
",
                sha256_hex(&served).unwrap()
            ),
        )
        .unwrap();
        assert!(apply_to(&rel(url(&sums)), &Kind::Portable(target.clone())).is_ok());
        assert_eq!(std::fs::read(&target).unwrap(), prog);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn replace_swaps_the_file_and_keeps_the_old_one_aside() {
        let dir = std::env::temp_dir().join(format!("cutline-replace-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (target, new) = (dir.join("cutline.exe"), dir.join("cutline.exe.new"));
        std::fs::write(&target, "old").unwrap();
        std::fs::write(&new, "new").unwrap();
        replace(&new, &target).unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");
        assert_eq!(
            std::fs::read_to_string(dir.join("cutline.exe.old")).unwrap(),
            "old"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
