//! A tiny software rasteriser for egui meshes.
//!
//! Export renders the annotations through a headless egui `Context` (so shapes and text look exactly
//! like on screen), tessellates them, and draws the triangles here straight into the cropped pixel
//! buffer. Nothing is ever shown on screen, so exporting cannot make the overlay flicker.

use eframe::egui::{
    ClippedPrimitive, Color32, ImageData, TextureFilter, TextureId, TexturesDelta,
    epaint::{Primitive, Vertex},
};
use std::collections::HashMap;

struct Tex {
    size: [usize; 2],
    pixels: Vec<Color32>,
    linear: bool,
}

#[derive(Default)]
pub struct Textures(HashMap<TextureId, Tex>);

impl Textures {
    /// Apply the texture updates a headless egui pass produced (font atlas, mosaic image, ...).
    pub fn apply(&mut self, delta: &TexturesDelta) {
        for (id, deltas) in &delta.set {
            for d in deltas {
                let ImageData::Color(img) = &d.image;
                let linear = d.options.magnification == TextureFilter::Linear;
                match d.pos {
                    None => {
                        self.0.insert(*id, Tex { size: img.size, pixels: img.pixels.clone(), linear });
                    }
                    Some([px, py]) => {
                        if let Some(t) = self.0.get_mut(id) {
                            for y in 0..img.size[1] {
                                let (ty, w) = (py + y, img.size[0]);
                                if ty >= t.size[1] {
                                    break;
                                }
                                let n = w.min(t.size[0].saturating_sub(px));
                                let (src, dst) = (y * w, ty * t.size[0] + px);
                                t.pixels[dst..dst + n].copy_from_slice(&img.pixels[src..src + n]);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn texel(t: &Tex, x: isize, y: isize) -> [f32; 4] {
    let x = x.clamp(0, t.size[0] as isize - 1) as usize;
    let y = y.clamp(0, t.size[1] as isize - 1) as usize;
    let [r, g, b, a] = t.pixels[y * t.size[0] + x].to_array();
    [r as f32, g as f32, b as f32, a as f32]
}

fn sample(t: &Tex, u: f32, v: f32) -> [f32; 4] {
    let (fx, fy) = (u * t.size[0] as f32, v * t.size[1] as f32);
    if !t.linear {
        return texel(t, fx.floor() as isize, fy.floor() as isize);
    }
    let (fx, fy) = (fx - 0.5, fy - 0.5);
    let (x0, y0) = (fx.floor(), fy.floor());
    let (tx, ty) = (fx - x0, fy - y0);
    let (x0, y0) = (x0 as isize, y0 as isize);
    let (a, b) = (texel(t, x0, y0), texel(t, x0 + 1, y0));
    let (c, d) = (texel(t, x0, y0 + 1), texel(t, x0 + 1, y0 + 1));
    let mut out = [0.0; 4];
    for i in 0..4 {
        let top = a[i] + (b[i] - a[i]) * tx;
        let bot = c[i] + (d[i] - c[i]) * tx;
        out[i] = top + (bot - top) * ty;
    }
    out
}

#[derive(Clone, Copy)]
struct P {
    x: f32,
    y: f32,
    u: f32,
    v: f32,
    c: [f32; 4],
}

fn edge(a: &P, b: &P, x: f32, y: f32) -> f32 {
    (b.x - a.x) * (y - a.y) - (b.y - a.y) * (x - a.x)
}

/// Which of the two triangles sharing an edge draws the pixels lying exactly on it.
fn owns(s: &P, t: &P) -> bool {
    let (dx, dy) = (t.x - s.x, t.y - s.y);
    dy > 0.0 || (dy == 0.0 && dx > 0.0)
}

/// Draw `prims` (positions in points) over the opaque RGBA buffer `dst` (`w` x `h` pixels).
/// A point `p` lands on pixel `(p - origin_pt) * ppp - offset_px`.
pub fn draw(
    dst: &mut [u8],
    w: usize,
    h: usize,
    ppp: f32,
    origin_pt: (f32, f32),
    offset_px: (f32, f32),
    prims: &[ClippedPrimitive],
    texs: &Textures,
) {
    let to_px = |x: f32, y: f32| ((x - origin_pt.0) * ppp - offset_px.0, (y - origin_pt.1) * ppp - offset_px.1);
    for prim in prims {
        let Primitive::Mesh(mesh) = &prim.primitive else { continue };
        let Some(tex) = texs.0.get(&mesh.texture_id) else { continue };

        let (cx0, cy0) = to_px(prim.clip_rect.min.x, prim.clip_rect.min.y);
        let (cx1, cy1) = to_px(prim.clip_rect.max.x, prim.clip_rect.max.y);
        let clip_x0 = cx0.round().clamp(0.0, w as f32) as usize;
        let clip_y0 = cy0.round().clamp(0.0, h as f32) as usize;
        let clip_x1 = cx1.round().clamp(0.0, w as f32) as usize;
        let clip_y1 = cy1.round().clamp(0.0, h as f32) as usize;
        if clip_x0 >= clip_x1 || clip_y0 >= clip_y1 {
            continue;
        }

        let pts: Vec<P> = mesh
            .vertices
            .iter()
            .map(|v: &Vertex| {
                let (x, y) = to_px(v.pos.x, v.pos.y);
                let [r, g, b, a] = v.color.to_array();
                P { x, y, u: v.uv.x, v: v.uv.y, c: [r as f32, g as f32, b as f32, a as f32] }
            })
            .collect();

        for tri in mesh.indices.chunks_exact(3) {
            let (a, mut b, mut c) = (pts[tri[0] as usize], pts[tri[1] as usize], pts[tri[2] as usize]);
            let mut area = edge(&a, &b, c.x, c.y);
            if area == 0.0 || !area.is_finite() {
                continue;
            }
            if area < 0.0 {
                std::mem::swap(&mut b, &mut c);
                area = -area;
            }
            let min_x = a.x.min(b.x).min(c.x).floor().max(clip_x0 as f32) as usize;
            let max_x = (a.x.max(b.x).max(c.x).ceil().max(0.0) as usize).min(clip_x1);
            let min_y = a.y.min(b.y).min(c.y).floor().max(clip_y0 as f32) as usize;
            let max_y = (a.y.max(b.y).max(c.y).ceil().max(0.0) as usize).min(clip_y1);
            let (own0, own1, own2) = (owns(&b, &c), owns(&c, &a), owns(&a, &b));
            let inv = 1.0 / area;
            for y in min_y..max_y {
                let py = y as f32 + 0.5;
                for x in min_x..max_x {
                    let px = x as f32 + 0.5;
                    let (w0, w1, w2) = (edge(&b, &c, px, py), edge(&c, &a, px, py), edge(&a, &b, px, py));
                    let inside = (w0 > 0.0 || (w0 == 0.0 && own0))
                        && (w1 > 0.0 || (w1 == 0.0 && own1))
                        && (w2 > 0.0 || (w2 == 0.0 && own2));
                    if !inside {
                        continue;
                    }
                    let (l0, l1, l2) = (w0 * inv, w1 * inv, w2 * inv);
                    let u = a.u * l0 + b.u * l1 + c.u * l2;
                    let v = a.v * l0 + b.v * l1 + c.v * l2;
                    let t = sample(tex, u, v);
                    let mut s = [0.0f32; 4];
                    for i in 0..4 {
                        let vc = a.c[i] * l0 + b.c[i] * l1 + c.c[i] * l2;
                        s[i] = vc * t[i] / 255.0;
                    }
                    let inv_a = 1.0 - s[3] / 255.0;
                    let o = (y * w + x) * 4;
                    for i in 0..3 {
                        dst[o + i] = (s[i] + dst[o + i] as f32 * inv_a).round().clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }
    }
}
