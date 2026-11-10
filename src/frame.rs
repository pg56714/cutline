//! A frozen screenshot of one monitor, handed from the resident daemon to the
//! overlay process through a temp file (raw RGBA, so no encode cost).
//!
//! Positions and sizes are in *logical points* (what the windowing system uses to
//! place windows); `width`/`height` are the captured image's pixels. The daemon does
//! the per-OS conversion (Windows/Linux report pixels, macOS reports points), so the
//! overlay never has to care.

use std::io::{Read, Write};
use std::path::PathBuf;

pub struct Frame {
    /// Monitor origin and size in logical points (virtual desktop space).
    pub x: f32,
    pub y: f32,
    pub lw: f32,
    pub lh: f32,
    /// Captured image size in pixels.
    pub width: u32,
    pub height: u32,
    /// Pixels per logical point.
    pub scale: f32,
    /// Top-level window rectangles `[x, y, w, h]` in logical points, topmost first.
    /// Used for hover-to-select-window.
    pub windows: Vec<[f32; 4]>,
    pub rgba: Vec<u8>,
}

pub fn temp_dir() -> PathBuf {
    let d = std::env::temp_dir().join("cutline");
    let _ = std::fs::create_dir_all(&d);
    d
}

impl Frame {
    pub fn write(&self, path: &std::path::Path) -> std::io::Result<()> {
        let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
        for v in [self.x, self.y, self.lw, self.lh] {
            f.write_all(&v.to_le_bytes())?;
        }
        f.write_all(&self.width.to_le_bytes())?;
        f.write_all(&self.height.to_le_bytes())?;
        f.write_all(&self.scale.to_le_bytes())?;
        f.write_all(&(self.windows.len() as u32).to_le_bytes())?;
        for w in &self.windows {
            for v in w {
                f.write_all(&v.to_le_bytes())?;
            }
        }
        f.write_all(&self.rgba)?;
        f.flush()
    }

    pub fn read(path: &std::path::Path) -> std::io::Result<Frame> {
        let mut f = std::io::BufReader::new(std::fs::File::open(path)?);
        let mut b = [0u8; 4];
        let mut f32_ = |f: &mut dyn Read| -> std::io::Result<f32> {
            f.read_exact(&mut b)?;
            Ok(f32::from_le_bytes(b))
        };
        let (x, y, lw, lh) = (f32_(&mut f)?, f32_(&mut f)?, f32_(&mut f)?, f32_(&mut f)?);
        let u32_ = |f: &mut dyn Read| -> std::io::Result<u32> {
            let mut b = [0u8; 4];
            f.read_exact(&mut b)?;
            Ok(u32::from_le_bytes(b))
        };
        let width = u32_(&mut f)?;
        let height = u32_(&mut f)?;
        let scale = {
            let mut b = [0u8; 4];
            f.read_exact(&mut b)?;
            f32::from_le_bytes(b)
        };
        let n = u32_(&mut f)? as usize;
        let mut windows = Vec::with_capacity(n.min(4096));
        for _ in 0..n {
            let mut w = [0f32; 4];
            for v in &mut w {
                let mut b = [0u8; 4];
                f.read_exact(&mut b)?;
                *v = f32::from_le_bytes(b);
            }
            windows.push(w);
        }
        let mut rgba = vec![0u8; (width as usize) * (height as usize) * 4];
        f.read_exact(&mut rgba)?;
        Ok(Frame { x, y, lw, lh, width, height, scale, windows, rgba })
    }
}
