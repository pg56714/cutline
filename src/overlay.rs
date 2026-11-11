//! The capture overlay: shows the frozen frame, lets you pick a region
//! (hover a window to snap to it, or drag), annotate it, then copy / save / pin.
//!
//! Export is rendered off screen: the selection is cropped from the frozen frame and the annotations
//! are drawn on top through a headless egui context + a small software rasteriser (`raster`).
//! The overlay itself never changes while exporting, so copy / save / pin cannot flicker.

#![allow(deprecated)] // egui 0.37 deprecates Id::new in favour of salts; harmless here.

use crate::frame::Frame;
use eframe::egui::{
    self, Align2, Color32, ColorImage, Context, CursorIcon, FontId, Id, Key, LayerId, Order, Painter,
    Pos2, Rect, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui,
    Vec2, ViewportBuilder, ViewportCommand, pos2, vec2,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

#[cfg(any(debug_assertions, feature = "devscript"))]
mod devscript;
mod raster;

const ACCENT: Color32 = Color32::from_rgb(30, 136, 229);
const BAR_BG: Color32 = Color32::from_rgb(247, 247, 247);
const BAR_LINE: Color32 = Color32::from_rgb(205, 205, 205);
const ICON: Color32 = Color32::from_rgb(70, 70, 70);
const ICON_OFF: Color32 = Color32::from_rgb(185, 185, 185);
// Snipaste-like: a light veil while picking, a darker one once a region is being dragged / selected.
const DIM_PICK: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 70);
const DIM: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 145);

const PALETTE: [Color32; 8] = [
    Color32::from_rgb(229, 57, 53),
    Color32::from_rgb(251, 140, 0),
    Color32::from_rgb(253, 216, 53),
    Color32::from_rgb(67, 160, 71),
    Color32::from_rgb(30, 136, 229),
    Color32::from_rgb(142, 36, 170),
    Color32::from_rgb(33, 33, 33),
    Color32::from_rgb(250, 250, 250),
];
const WIDTHS: [f32; 3] = [2.0, 4.0, 7.0];
const TEXT_SIZES: [f32; 3] = [16.0, 24.0, 36.0];
const MOSAIC_BLOCK: u32 = 10;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tool {
    Shape,
    Arrow,
    Pen,
    Highlighter,
    Mosaic,
    Text,
    Eraser,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Btn {
    Tool(Tool),
    Undo,
    Redo,
    Close,
    Pin,
    Save,
    Copy,
    Sep,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Sub {
    Kind(usize),
    Width(usize),
    Color(usize),
    Sep,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Copy,
    Save,
    Pin,
}

#[derive(Clone, Debug)]
enum Annot {
    Rect { r: Rect, c: Color32, w: f32, ellipse: bool },
    Line { a: Pos2, b: Pos2, c: Color32, w: f32, arrow: bool },
    Pen { pts: Vec<Pos2>, c: Color32, w: f32 },
    Highlight { pts: Vec<Pos2>, c: Color32, w: f32 },
    Mosaic { r: Rect },
    Text { pos: Pos2, text: String, c: Color32, size: f32, wrap: f32, bounds: Rect },
}

enum Drag {
    None,
    NewSel { start: Pos2 },
    Move { grab: Vec2 },
    Resize { h: (i8, i8), orig: Rect },
    Draw { start: Pos2 },
    Erase,
    /// Dragging the text box's right-edge handle; `grab` = pointer x minus the box's right edge.
    TextWidth { grab: f32 },
}

struct Editing {
    pos: Pos2,
    text: String,
    c: Color32,
    size: f32,
    /// Wrap width in points; adjustable by dragging the box's right edge.
    width: f32,
}

const TEXT_MIN_W: f32 = 40.0;
const TEXT_DEFAULT_W: f32 = 180.0;

/// A save dialog is open (it runs on its own thread).
struct Export {
    /// Dialog result: 0 pending, 1 saved, 2 cancelled.
    save_state: Arc<AtomicU8>,
}

struct Overlay {
    frame: Frame,
    tex: TextureHandle,
    mosaic_tex: Option<TextureHandle>,
    screen: Rect,
    /// Top-left of this window in logical desktop points (for placing pins).
    win_pos: Pos2,
    windows: Vec<Rect>,

    sel: Option<Rect>,
    live: Option<Rect>,
    drag: Drag,
    press_btn: Option<Btn>,
    press_sub: Option<Sub>,

    tool: Option<Tool>,
    shape_ellipse: bool,
    arrow_line: bool,
    width_idx: usize,
    color_idx: usize,
    size_idx: usize,

    shapes: Vec<Annot>,
    undo: Vec<Vec<Annot>>,
    redo: Vec<Vec<Annot>>,
    cur: Option<Annot>,
    editing: Option<Editing>,
    edit_rect: Rect,

    export: Option<Export>,
    save_job: Option<(u32, u32, Vec<u8>)>,
    frames: u32,
    #[cfg(any(debug_assertions, feature = "devscript"))]
    script: Option<devscript::Script>,
}

pub fn run(args: &[String]) {
    let Some(path) = args.first().map(std::path::PathBuf::from) else { return };
    let Ok(frame) = Frame::read(&path) else { return };
    let _ = std::fs::remove_file(&path);

    let pos = pos2(frame.x, frame.y);
    let size = vec2(frame.lw, frame.lh);
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("Cutline")
            .with_decorations(false)
            .with_always_on_top()
            .with_visible(false) // shown once the first frame is painted: no white flash
            .with_taskbar(false)
            .with_resizable(false)
            .with_position(pos)
            .with_inner_size(size),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Cutline",
        options,
        Box::new(move |cc| Ok(Box::new(Overlay::new(cc, frame, pos, size)))),
    );
}

impl Overlay {
    fn new(cc: &eframe::CreationContext<'_>, frame: Frame, win_pos: Pos2, size: Vec2) -> Self {
        let ci = ColorImage::from_rgba_unmultiplied([frame.width as usize, frame.height as usize], &frame.rgba);
        let tex = cc.egui_ctx.load_texture("shot", ci, TextureOptions::NEAREST);
        let screen = Rect::from_min_size(Pos2::ZERO, size);
        let windows = frame
            .windows
            .iter()
            .filter_map(|w| {
                let r = Rect::from_min_size(pos2(w[0] - frame.x, w[1] - frame.y), vec2(w[2], w[3])).intersect(screen);
                (r.width() > 8.0 && r.height() > 8.0).then_some(r)
            })
            .collect();
        Overlay {
            frame,
            tex,
            mosaic_tex: None,
            screen,
            win_pos,
            windows,
            sel: None,
            live: None,
            drag: Drag::None,
            press_btn: None,
            press_sub: None,
            tool: None,
            shape_ellipse: false,
            arrow_line: false,
            width_idx: 1,
            color_idx: 0,
            size_idx: 1,
            shapes: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            cur: None,
            editing: None,
            edit_rect: Rect::NOTHING,
            export: None,
            save_job: None,
            frames: 0,
            #[cfg(any(debug_assertions, feature = "devscript"))]
            script: std::env::var("CUTLINE_SCRIPT").ok().map(|s| devscript::parse(&s)),
        }
    }

    // ---- geometry helpers -------------------------------------------------

    /// Physical image pixels per logical point.
    fn k(&self) -> f32 {
        self.frame.width as f32 / self.screen.width().max(1.0)
    }

    fn pixel_at(&self, p: Pos2) -> (i32, i32) {
        let k = self.k();
        let x = ((p.x - self.screen.min.x) * k).floor() as i32;
        let y = ((p.y - self.screen.min.y) * k).floor() as i32;
        (x.clamp(0, self.frame.width as i32 - 1), y.clamp(0, self.frame.height as i32 - 1))
    }

    fn rgb_at(&self, p: Pos2) -> [u8; 3] {
        let (x, y) = self.pixel_at(p);
        let i = ((y as usize) * self.frame.width as usize + x as usize) * 4;
        [self.frame.rgba[i], self.frame.rgba[i + 1], self.frame.rgba[i + 2]]
    }

    fn window_at(&self, p: Pos2) -> Option<Rect> {
        self.windows.iter().find(|r| r.contains(p)).copied()
    }

    fn color(&self) -> Color32 {
        PALETTE[self.color_idx]
    }

    // ---- toolbar layout ---------------------------------------------------

    fn bar_items() -> Vec<Btn> {
        vec![
            Btn::Tool(Tool::Shape),
            Btn::Tool(Tool::Arrow),
            Btn::Tool(Tool::Pen),
            Btn::Tool(Tool::Highlighter),
            Btn::Tool(Tool::Mosaic),
            Btn::Tool(Tool::Text),
            Btn::Tool(Tool::Eraser),
            Btn::Sep,
            Btn::Undo,
            Btn::Redo,
            Btn::Sep,
            Btn::Close,
            Btn::Pin,
            Btn::Save,
            Btn::Copy,
        ]
    }

    /// Returns (bar rect, buttons, bar is below the selection).
    fn layout_bar(&self, sel: Rect) -> (Rect, Vec<(Rect, Btn)>, bool) {
        const BW: f32 = 34.0;
        const BH: f32 = 32.0;
        const SW: f32 = 9.0;
        let items = Self::bar_items();
        let total: f32 = items.iter().map(|b| if *b == Btn::Sep { SW } else { BW }).sum::<f32>() + 8.0;
        let h = BH + 6.0;
        let x = (sel.max.x - total).clamp(self.screen.min.x + 4.0, (self.screen.max.x - total - 4.0).max(self.screen.min.x));
        let below = sel.max.y + 8.0 + h + 40.0 <= self.screen.max.y;
        let y = if below {
            sel.max.y + 8.0
        } else if sel.min.y - 8.0 - h - 40.0 >= self.screen.min.y {
            sel.min.y - 8.0 - h
        } else {
            (sel.max.y - h - 8.0).max(self.screen.min.y + 4.0)
        };
        let bar = Rect::from_min_size(pos2(x, y), vec2(total, h));
        let mut cx = x + 4.0;
        let mut out = Vec::new();
        for b in items {
            let w = if b == Btn::Sep { SW } else { BW };
            out.push((Rect::from_min_size(pos2(cx, y + 3.0), vec2(w, BH)), b));
            cx += w;
        }
        (bar, out, below)
    }

    fn sub_items(&self) -> Vec<Sub> {
        let mut v = Vec::new();
        let Some(t) = self.tool else { return v };
        match t {
            Tool::Shape | Tool::Arrow => {
                v.extend([Sub::Kind(0), Sub::Kind(1), Sub::Sep]);
                v.extend((0..3).map(Sub::Width));
                v.push(Sub::Sep);
                v.extend((0..PALETTE.len()).map(Sub::Color));
            }
            Tool::Pen | Tool::Highlighter => {
                v.extend((0..3).map(Sub::Width));
                v.push(Sub::Sep);
                v.extend((0..PALETTE.len()).map(Sub::Color));
            }
            Tool::Text => {
                v.extend((0..3).map(Sub::Width));
                v.push(Sub::Sep);
                v.extend((0..PALETTE.len()).map(Sub::Color));
            }
            Tool::Mosaic | Tool::Eraser => {}
        }
        v
    }

    fn layout_sub(&self, bar: Rect, below: bool) -> Option<(Rect, Vec<(Rect, Sub)>)> {
        let items = self.sub_items();
        if items.is_empty() {
            return None;
        }
        const C: f32 = 26.0;
        const S: f32 = 9.0;
        let total: f32 = items.iter().map(|s| if *s == Sub::Sep { S } else { C }).sum::<f32>() + 8.0;
        let h = C + 6.0;
        let x = bar.min.x.min(self.screen.max.x - total - 4.0).max(self.screen.min.x + 4.0);
        let y = if below { bar.max.y + 4.0 } else { bar.min.y - 4.0 - h };
        let rect = Rect::from_min_size(pos2(x, y), vec2(total, h));
        let mut cx = x + 4.0;
        let mut out = Vec::new();
        for s in items {
            let w = if s == Sub::Sep { S } else { C };
            out.push((Rect::from_min_size(pos2(cx, y + 3.0), vec2(w, C)), s));
            cx += w;
        }
        Some((rect, out))
    }

    // ---- history ----------------------------------------------------------

    fn push_history(&mut self) {
        self.undo.push(self.shapes.clone());
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn do_undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.shapes, prev));
        }
    }

    fn do_redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.shapes, next));
        }
    }

    /// Grab zone for resizing the text box: a strip just right of (and slightly over) its edge.
    fn text_handle(&self) -> Rect {
        let r = self.edit_rect;
        Rect::from_min_max(pos2(r.max.x - 1.0, r.min.y - 3.0), pos2(r.max.x + 11.0, r.max.y + 3.0))
    }

    fn commit_text(&mut self, ctx: &Context) {
        if let Some(e) = self.editing.take() {
            if !e.text.trim().is_empty() {
                let galley = ctx.layer_painter(LayerId::background()).layout(
                    e.text.clone(),
                    FontId::proportional(e.size),
                    e.c,
                    e.width,
                );
                let bounds = Rect::from_min_size(e.pos, galley.size());
                self.push_history();
                self.shapes.push(Annot::Text { pos: e.pos, text: e.text, c: e.c, size: e.size, wrap: e.width, bounds });
            }
        }
    }

    // ---- input ------------------------------------------------------------

    fn ensure_mosaic(&mut self, ctx: &Context) {
        if self.mosaic_tex.is_some() {
            return;
        }
        let ci = self.mosaic_image();
        self.mosaic_tex = Some(ctx.load_texture("mosaic", ci, TextureOptions::NEAREST));
    }

    /// The whole frame pixelated in `MOSAIC_BLOCK` squares.
    fn mosaic_image(&self) -> ColorImage {
        let (w, h) = (self.frame.width, self.frame.height);
        let mut out = vec![0u8; (w * h * 4) as usize];
        let mut by = 0;
        while by < h {
            let bh = MOSAIC_BLOCK.min(h - by);
            let mut bx = 0;
            while bx < w {
                let bw = MOSAIC_BLOCK.min(w - bx);
                let (mut r, mut g, mut b) = (0u32, 0u32, 0u32);
                for y in by..by + bh {
                    for x in bx..bx + bw {
                        let i = ((y * w + x) * 4) as usize;
                        r += self.frame.rgba[i] as u32;
                        g += self.frame.rgba[i + 1] as u32;
                        b += self.frame.rgba[i + 2] as u32;
                    }
                }
                let n = bw * bh;
                let (r, g, b) = ((r / n) as u8, (g / n) as u8, (b / n) as u8);
                for y in by..by + bh {
                    for x in bx..bx + bw {
                        let i = ((y * w + x) * 4) as usize;
                        out[i..i + 4].copy_from_slice(&[r, g, b, 255]);
                    }
                }
                bx += MOSAIC_BLOCK;
            }
            by += MOSAIC_BLOCK;
        }
        ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &out)
    }

    fn clamp_to(r: Rect, p: Pos2) -> Pos2 {
        pos2(p.x.clamp(r.min.x, r.max.x), p.y.clamp(r.min.y, r.max.y))
    }

    fn begin_draw(&mut self, ctx: &Context, t: Tool, p: Pos2) {
        let c = self.color();
        match t {
            Tool::Shape => {
                self.cur = Some(Annot::Rect {
                    r: Rect::from_two_pos(p, p),
                    c,
                    w: WIDTHS[self.width_idx],
                    ellipse: self.shape_ellipse,
                });
                self.drag = Drag::Draw { start: p };
            }
            Tool::Arrow => {
                self.cur = Some(Annot::Line { a: p, b: p, c, w: WIDTHS[self.width_idx], arrow: !self.arrow_line });
                self.drag = Drag::Draw { start: p };
            }
            Tool::Pen => {
                self.cur = Some(Annot::Pen { pts: vec![p], c, w: WIDTHS[self.width_idx] });
                self.drag = Drag::Draw { start: p };
            }
            Tool::Highlighter => {
                self.cur = Some(Annot::Highlight { pts: vec![p], c, w: WIDTHS[self.width_idx] * 4.0 });
                self.drag = Drag::Draw { start: p };
            }
            Tool::Mosaic => {
                self.ensure_mosaic(ctx);
                self.cur = Some(Annot::Mosaic { r: Rect::from_two_pos(p, p) });
                self.drag = Drag::Draw { start: p };
            }
            Tool::Text => {
                self.editing = Some(Editing { pos: p, text: String::new(), c, size: TEXT_SIZES[self.size_idx], width: TEXT_DEFAULT_W });
                self.edit_rect = Rect::from_min_size(p, vec2(4.0, 4.0));
                self.drag = Drag::None;
            }
            Tool::Eraser => {
                self.push_history();
                self.drag = Drag::Erase;
            }
        }
    }

    fn update_draw(&mut self, start: Pos2, p: Pos2, shift: bool) {
        let Some(cur) = self.cur.as_mut() else { return };
        match cur {
            Annot::Rect { r, .. } => {
                let mut q = p;
                if shift {
                    let d = p - start;
                    let side = d.x.abs().max(d.y.abs());
                    q = start + vec2(side * d.x.signum(), side * d.y.signum());
                }
                *r = Rect::from_two_pos(start, q);
            }
            Annot::Line { a, b, .. } => {
                *a = start;
                let mut q = p;
                if shift {
                    let d = p - start;
                    let len = d.length();
                    let ang = (d.y.atan2(d.x) / std::f32::consts::FRAC_PI_4).round() * std::f32::consts::FRAC_PI_4;
                    q = start + vec2(ang.cos(), ang.sin()) * len;
                }
                *b = q;
            }
            Annot::Pen { pts, .. } | Annot::Highlight { pts, .. } => {
                if pts.last().map(|l| l.distance(p) > 0.8).unwrap_or(true) {
                    pts.push(p);
                }
            }
            Annot::Mosaic { r } => *r = Rect::from_two_pos(start, p),
            Annot::Text { .. } => {}
        }
    }

    fn finish_draw(&mut self) {
        if let Some(cur) = self.cur.take() {
            let keep = match &cur {
                Annot::Rect { r, .. } | Annot::Mosaic { r } => r.width() > 2.0 && r.height() > 2.0,
                Annot::Line { a, b, .. } => a.distance(*b) > 3.0,
                Annot::Pen { pts, .. } | Annot::Highlight { pts, .. } => pts.len() > 1,
                Annot::Text { .. } => true,
            };
            if keep {
                self.push_history();
                self.shapes.push(cur);
            }
        }
    }

    fn handle_at(sel: Rect, p: Pos2) -> Option<(i8, i8)> {
        let tol = 7.0;
        let xs = [(sel.min.x, -1i8), (sel.center().x, 0), (sel.max.x, 1)];
        let ys = [(sel.min.y, -1i8), (sel.center().y, 0), (sel.max.y, 1)];
        for (y, hy) in ys {
            for (x, hx) in xs {
                if hx == 0 && hy == 0 {
                    continue;
                }
                if (p.x - x).abs() <= tol && (p.y - y).abs() <= tol {
                    return Some((hx, hy));
                }
            }
        }
        None
    }

    fn resize(orig: Rect, h: (i8, i8), p: Pos2, screen: Rect) -> Rect {
        let (mut min, mut max) = (orig.min, orig.max);
        match h.0 {
            -1 => min.x = p.x,
            1 => max.x = p.x,
            _ => {}
        }
        match h.1 {
            -1 => min.y = p.y,
            1 => max.y = p.y,
            _ => {}
        }
        Rect::from_two_pos(min, max).intersect(screen)
    }

    fn erase_at(&mut self, p: Pos2) {
        let tol = 7.0;
        self.shapes.retain(|s| !annot_hit(s, p, tol));
    }

    fn start_export(&mut self, ctx: &Context, action: Action) {
        if self.sel.is_none() || self.export.is_some() {
            return;
        }
        self.commit_text(ctx);
        let Some((w, h, rgba)) = self.render_selection() else { return };
        match action {
            Action::Copy => {
                // Scripted dev runs write a file instead of touching the real clipboard.
                #[cfg(any(debug_assertions, feature = "devscript"))]
                let scripted = self.script.is_some() && std::env::var_os("CUTLINE_REAL_CLIPBOARD").is_none();
                #[cfg(not(any(debug_assertions, feature = "devscript")))]
                let scripted = false;
                if scripted {
                    if let Some(im) = image::RgbaImage::from_raw(w, h, rgba) {
                        let _ = im.save(crate::frame::temp_dir().join("export.png"));
                    }
                } else if let Ok(mut cb) = arboard::Clipboard::new() {
                    let _ = cb.set_image(arboard::ImageData { width: w as usize, height: h as usize, bytes: rgba.into() });
                }
                close_overlay(ctx);
            }
            Action::Pin => match self.spawn_pin(w, h, rgba) {
                Some(mut child) => {
                    // Keep the overlay on screen until the pin window has painted its first frame,
                    // then hide it: the pin takes over at the same spot with no gap.
                    let state = Arc::new(AtomicU8::new(0));
                    self.export = Some(Export { save_state: state.clone() });
                    let c2 = ctx.clone();
                    std::thread::spawn(move || {
                        if let Some(out) = child.stdout.take() {
                            let _ = std::io::BufRead::read_line(&mut std::io::BufReader::new(out), &mut String::new());
                        }
                        state.store(1, Ordering::SeqCst);
                        c2.request_repaint();
                    });
                }
                None => close_overlay(ctx),
            },
            Action::Save => {
                // The dialog needs the window handle, which only `ui()` has: hand the pixels over.
                self.save_job = Some((w, h, rgba));
                self.export = Some(Export { save_state: Arc::new(AtomicU8::new(0)) });
            }
        }
        ctx.request_repaint();
    }

    /// Open the save dialog as a child of the overlay window, so it shows above the (topmost)
    /// overlay without the overlay having to change anything on screen.
    fn begin_save(&mut self, ctx: &Context, frame: &eframe::Frame, (w, h, rgba): (u32, u32, Vec<u8>)) {
        let Some(ex) = self.export.as_ref() else { return };
        let state = ex.save_state.clone();
        let name = format!("Cutline_{}.png", chrono::Local::now().format("%Y%m%d_%H%M%S"));
        let mut dlg = rfd::FileDialog::new().set_file_name(name).add_filter("PNG image", &["png"]).set_parent(frame);
        if let Some(home) = std::env::var_os("USERPROFILE") {
            let pics = std::path::PathBuf::from(home).join("Pictures");
            if pics.is_dir() {
                dlg = dlg.set_directory(pics);
            }
        }
        let c2 = ctx.clone();
        std::thread::spawn(move || {
            let saved = dlg.save_file().and_then(|mut path| {
                if path.extension().is_none() {
                    path.set_extension("png");
                }
                image::RgbaImage::from_raw(w, h, rgba).and_then(|im| im.save(path).ok())
            });
            state.store(if saved.is_some() { 1 } else { 2 }, Ordering::SeqCst);
            c2.request_repaint();
        });
    }

    /// While the save dialog is open: close on success, resume editing on cancel.
    fn poll_save(&mut self, ctx: &Context) {
        let Some(ex) = self.export.as_ref() else { return };
        match ex.save_state.load(Ordering::SeqCst) {
            1 => close_overlay(ctx),
            2 => {
                self.export = None;
                ctx.send_viewport_cmd(ViewportCommand::Focus);
            }
            _ => {}
        }
    }

    fn click_btn(&mut self, ctx: &Context, b: Btn) {
        match b {
            Btn::Tool(t) => {
                self.commit_text(ctx);
                self.tool = if self.tool == Some(t) { None } else { Some(t) };
            }
            Btn::Undo => {
                self.commit_text(ctx);
                self.do_undo();
            }
            Btn::Redo => self.do_redo(),
            Btn::Close => close_overlay(ctx),
            Btn::Pin => self.start_export(ctx, Action::Pin),
            Btn::Save => self.start_export(ctx, Action::Save),
            Btn::Copy => self.start_export(ctx, Action::Copy),
            Btn::Sep => {}
        }
    }

    fn click_sub(&mut self, s: Sub) {
        match (s, self.tool) {
            (Sub::Kind(k), Some(Tool::Shape)) => self.shape_ellipse = k == 1,
            (Sub::Kind(k), Some(Tool::Arrow)) => self.arrow_line = k == 1,
            (Sub::Width(i), Some(Tool::Text)) => self.size_idx = i,
            (Sub::Width(i), _) => self.width_idx = i,
            (Sub::Color(i), _) => self.color_idx = i,
            _ => {}
        }
    }

    fn handle_input(&mut self, ctx: &Context) {
        struct In {
            pos: Option<Pos2>,
            pressed: bool,
            released: bool,
            down: bool,
            secondary: bool,
            double: bool,
            shift: bool,
            ctrl: bool,
            esc: bool,
            enter: bool,
            copy_event: bool,
            key_c: bool,
            key_s: bool,
            key_z: bool,
            key_y: bool,
            key_f3: bool,
        }
        let i = ctx.input(|i| In {
            pos: i.pointer.latest_pos(),
            pressed: i.pointer.button_pressed(egui::PointerButton::Primary),
            released: i.pointer.button_released(egui::PointerButton::Primary),
            down: i.pointer.button_down(egui::PointerButton::Primary),
            secondary: i.pointer.button_pressed(egui::PointerButton::Secondary),
            double: i.pointer.button_double_clicked(egui::PointerButton::Primary),
            shift: i.modifiers.shift,
            ctrl: i.modifiers.command,
            esc: i.key_pressed(Key::Escape),
            enter: i.key_pressed(Key::Enter),
            copy_event: i.events.iter().any(|e| matches!(e, egui::Event::Copy)),
            key_c: i.key_pressed(Key::C),
            key_s: i.key_pressed(Key::S),
            key_z: i.key_pressed(Key::Z),
            key_y: i.key_pressed(Key::Y),
            key_f3: i.key_pressed(Key::F3),
        });

        // --- keyboard ---
        if i.esc {
            if self.editing.is_some() {
                self.commit_text(ctx);
            } else {
                close_overlay(ctx);
                return;
            }
        }
        if self.editing.is_none() {
            if i.ctrl && i.key_z {
                self.do_undo();
            }
            if i.ctrl && i.key_y {
                self.do_redo();
            }
            if (i.enter || i.copy_event) && self.sel.is_some() {
                self.start_export(ctx, Action::Copy);
            }
            if i.ctrl && i.key_s {
                self.start_export(ctx, Action::Save);
            }
            if i.key_f3 {
                self.start_export(ctx, Action::Pin);
            }
            // C: copy the colour under the cursor (while picking a region).
            if i.key_c && !i.ctrl && self.sel.is_none() {
                if let Some(p) = i.pos {
                    let [r, g, b] = self.rgb_at(p);
                    let hex = format!("#{r:02X}{g:02X}{b:02X}");
                    #[cfg(any(debug_assertions, feature = "devscript"))]
                    let scripted = self.script.is_some() && std::env::var_os("CUTLINE_REAL_CLIPBOARD").is_none();
                    #[cfg(not(any(debug_assertions, feature = "devscript")))]
                    let scripted = false;
                    if scripted {
                        let _ = std::fs::write(crate::frame::temp_dir().join("color.txt"), &hex);
                    } else if let Ok(mut cb) = arboard::Clipboard::new() {
                        let _ = cb.set_text(hex);
                    }
                }
            }
        }

        let Some(p) = i.pos else { return };

        // --- right click: step back / quit ---
        if i.secondary {
            if self.editing.is_some() {
                self.commit_text(ctx);
            } else if self.sel.is_some() {
                self.sel = None;
                self.tool = None;
                self.shapes.clear();
                self.undo.clear();
                self.redo.clear();
                self.drag = Drag::None;
            } else {
                close_overlay(ctx);
            }
            return;
        }

        // --- press ---
        if i.pressed {
            self.press_btn = None;
            self.press_sub = None;
            if self.editing.is_some() {
                if self.text_handle().contains(p) {
                    self.drag = Drag::TextWidth { grab: p.x - self.edit_rect.max.x };
                } else if !self.edit_rect.expand(4.0).contains(p) {
                    self.commit_text(ctx);
                }
                return;
            }
            match self.sel {
                None => self.drag = Drag::NewSel { start: p },
                Some(sel) => {
                    let (bar, btns, below) = self.layout_bar(sel);
                    let sub = self.layout_sub(bar, below);
                    if let Some((_, b)) = btns.iter().find(|(r, b)| *b != Btn::Sep && r.contains(p)) {
                        self.press_btn = Some(*b);
                    } else if let Some((_, items)) = &sub {
                        self.press_sub = items.iter().find(|(r, s)| *s != Sub::Sep && r.contains(p)).map(|(_, s)| *s);
                    }
                    let on_bar = bar.contains(p) || sub.as_ref().map(|(r, _)| r.contains(p)).unwrap_or(false);
                    if !on_bar {
                        if let Some(h) = Self::handle_at(sel, p) {
                            self.drag = Drag::Resize { h, orig: sel };
                        } else if sel.contains(p) {
                            match self.tool {
                                None => self.drag = Drag::Move { grab: p - sel.min },
                                Some(t) => self.begin_draw(ctx, t, Self::clamp_to(sel, p)),
                            }
                        }
                    }
                }
            }
        }

        // --- drag ---
        if i.down {
            match &self.drag {
                Drag::NewSel { start } => {
                    self.live = Some(Rect::from_two_pos(*start, p).intersect(self.screen));
                }
                Drag::Resize { h, orig } => {
                    self.sel = Some(Self::resize(*orig, *h, Self::clamp_to(self.screen, p), self.screen));
                }
                Drag::Move { grab } => {
                    if let Some(sel) = self.sel {
                        let mut min = p - *grab;
                        min.x = min.x.clamp(self.screen.min.x, self.screen.max.x - sel.width());
                        min.y = min.y.clamp(self.screen.min.y, self.screen.max.y - sel.height());
                        let new = Rect::from_min_size(min, sel.size());
                        let d = new.min - sel.min;
                        // Annotations travel with the selection.
                        for s in &mut self.shapes {
                            annot_translate(s, d);
                        }
                        self.sel = Some(new);
                    }
                }
                Drag::Draw { start } => {
                    let start = *start;
                    if let Some(sel) = self.sel {
                        self.update_draw(start, Self::clamp_to(sel, p), i.shift);
                    }
                }
                Drag::Erase => self.erase_at(p),
                Drag::TextWidth { grab } => {
                    let grab = *grab;
                    if let Some(e) = self.editing.as_mut() {
                        let max_w = (self.screen.max.x - e.pos.x - 12.0).max(TEXT_MIN_W);
                        e.width = (p.x - grab - e.pos.x).clamp(TEXT_MIN_W, max_w);
                    }
                }
                Drag::None => {}
            }
        }

        // --- release ---
        if i.released {
            match std::mem::replace(&mut self.drag, Drag::None) {
                Drag::NewSel { start } => {
                    let r = if start.distance(p) < 4.0 {
                        self.window_at(p).unwrap_or(self.screen)
                    } else {
                        Rect::from_two_pos(start, p).intersect(self.screen)
                    };
                    if r.width() >= 3.0 && r.height() >= 3.0 {
                        self.sel = Some(r);
                    }
                    self.live = None;
                }
                Drag::Draw { .. } => self.finish_draw(),
                _ => {}
            }
            if let Some(b) = self.press_btn.take() {
                if let Some(sel) = self.sel {
                    let (_, btns, _) = self.layout_bar(sel);
                    if btns.iter().any(|(r, bb)| *bb == b && r.contains(p)) {
                        self.click_btn(ctx, b);
                    }
                }
            }
            if let Some(s) = self.press_sub.take() {
                if let Some(sel) = self.sel {
                    let (bar, _, below) = self.layout_bar(sel);
                    if let Some((_, items)) = self.layout_sub(bar, below) {
                        if items.iter().any(|(r, ss)| *ss == s && r.contains(p)) {
                            self.click_sub(s);
                        }
                    }
                }
            }
        }

        // Double-click inside the selection (no tool): copy and finish.
        if i.double && self.tool.is_none() {
            if let Some(sel) = self.sel {
                let (bar, _, below) = self.layout_bar(sel);
                let on_sub = self.layout_sub(bar, below).map(|(r, _)| r.contains(p)).unwrap_or(false);
                if sel.contains(p) && !bar.contains(p) && !on_sub {
                    self.start_export(ctx, Action::Copy);
                }
            }
        }

        // --- cursor ---
        let on_text_handle = self.editing.is_some() && self.text_handle().contains(p);
        let icon = match (&self.drag, self.sel) {
            (Drag::TextWidth { .. }, _) => CursorIcon::ResizeHorizontal,
            _ if on_text_handle => CursorIcon::ResizeHorizontal,
            (Drag::Move { .. }, _) => CursorIcon::Grabbing,
            (_, None) => CursorIcon::Crosshair,
            (_, Some(sel)) => {
                let (bar, _, below) = self.layout_bar(sel);
                let on_ui = bar.contains(p)
                    || self.layout_sub(bar, below).map(|(r, _)| r.contains(p)).unwrap_or(false);
                if on_ui {
                    CursorIcon::PointingHand
                } else if let Some(h) = Self::handle_at(sel, p) {
                    match h {
                        (-1, -1) | (1, 1) => CursorIcon::ResizeNwSe,
                        (1, -1) | (-1, 1) => CursorIcon::ResizeNeSw,
                        (0, _) => CursorIcon::ResizeVertical,
                        _ => CursorIcon::ResizeHorizontal,
                    }
                } else if sel.contains(p) {
                    match self.tool {
                        Some(Tool::Text) => CursorIcon::Text,
                        Some(_) => CursorIcon::Crosshair,
                        None => CursorIcon::Grab,
                    }
                } else {
                    CursorIcon::Default
                }
            }
        };
        ctx.set_cursor_icon(icon);
    }

    // ---- painting ---------------------------------------------------------

    fn paint_annots(&self, p: &Painter) {
        let mosaic = self.mosaic_tex.as_ref().map(|t| t.id());
        for s in &self.shapes {
            paint_annot(p, s, mosaic, self.screen);
        }
        if let Some(c) = &self.cur {
            paint_annot(p, c, mosaic, self.screen);
        }
    }

    fn paint_chrome(&mut self, ctx: &Context, p: &Painter, pos: Option<Pos2>) {
        let screen = self.screen;
        let region = self.live.or(self.sel);

        // Dim everything outside the region (or the hovered window while picking).
        let hover = if region.is_none() { pos.and_then(|q| self.window_at(q)) } else { None };
        let bright = region.or(hover);
        match bright {
            Some(r) => {
                let dim = if region.is_some() { DIM } else { DIM_PICK };
                p.rect_filled(Rect::from_min_max(screen.min, pos2(screen.max.x, r.min.y)), 0.0, dim);
                p.rect_filled(Rect::from_min_max(pos2(screen.min.x, r.max.y), screen.max), 0.0, dim);
                p.rect_filled(Rect::from_min_max(pos2(screen.min.x, r.min.y), pos2(r.min.x, r.max.y)), 0.0, dim);
                p.rect_filled(Rect::from_min_max(pos2(r.max.x, r.min.y), pos2(screen.max.x, r.max.y)), 0.0, dim);
            }
            None => {
                p.rect_filled(screen, 0.0, DIM_PICK);
            }
        }

        if let Some(r) = hover {
            p.rect_stroke(r, 0.0, Stroke::new(2.0, ACCENT), StrokeKind::Inside);
        }

        // Annotations are clipped to the selection.
        if let Some(sel) = self.sel {
            self.paint_annots(&p.with_clip_rect(sel));
        }

        if let Some(r) = region {
            p.rect_stroke(r, 0.0, Stroke::new(1.5, ACCENT), StrokeKind::Outside);
            // Size label.
            let k = self.k();
            let label = format!("{} × {}", (r.width() * k).round() as i32, (r.height() * k).round() as i32);
            let font = FontId::proportional(12.0);
            let galley = p.layout_no_wrap(label, font, Color32::WHITE);
            let size = galley.size() + vec2(12.0, 6.0);
            let y = if r.min.y - size.y - 4.0 >= screen.min.y { r.min.y - size.y - 4.0 } else { r.min.y + 4.0 };
            let lr = Rect::from_min_size(pos2(r.min.x.max(screen.min.x), y), size);
            p.rect_filled(lr, 3.0, Color32::from_black_alpha(190));
            p.galley(lr.min + vec2(6.0, 3.0), galley, Color32::WHITE);
        }

        // Handles + toolbar once a region is committed.
        if let (Some(sel), true) = (self.sel, self.live.is_none()) {
            for hy in [-1i8, 0, 1] {
                for hx in [-1i8, 0, 1] {
                    if hx == 0 && hy == 0 {
                        continue;
                    }
                    let c = pos2(
                        match hx { -1 => sel.min.x, 0 => sel.center().x, _ => sel.max.x },
                        match hy { -1 => sel.min.y, 0 => sel.center().y, _ => sel.max.y },
                    );
                    let hr = Rect::from_center_size(c, vec2(7.0, 7.0));
                    p.rect_filled(hr, 0.0, Color32::WHITE);
                    p.rect_stroke(hr, 0.0, Stroke::new(1.0, ACCENT), StrokeKind::Inside);
                }
            }
            self.paint_bars(p, pos, sel);
        }

        // Magnifier while picking.
        if self.sel.is_none() || self.live.is_some() {
            if let Some(q) = pos {
                self.paint_magnifier(p, q);
            }
        }

        // Text being typed.
        let mut commit = false;
        if let Some(e) = self.editing.as_mut() {
            let font = FontId::proportional(e.size);
            let color = e.c;
            let area = egui::Area::new(Id::new("cutline_text"))
                .order(Order::Foreground)
                .fixed_pos(e.pos)
                .show(ctx, |ui| {
                    let te = egui::TextEdit::multiline(&mut e.text)
                        .font(font)
                        .text_color(color)
                        .frame(egui::Frame::NONE)
                        .desired_width(e.width)
                        .desired_rows(1)
                        .lock_focus(true);
                    // The Area's max width is last frame's size, so give the editor its own width.
                    let r = ui.allocate_ui(vec2(e.width, 0.0), |ui| ui.add(te)).inner;
                    r.request_focus();
                    r.rect
                });
            let text_rect = area.inner;
            self.edit_rect = text_rect;
            let box_rect = text_rect.expand(3.0);
            p.rect_filled(box_rect, 2.0, Color32::from_rgba_unmultiplied(0, 0, 0, 90));
            p.rect_stroke(box_rect, 2.0, Stroke::new(1.5, ACCENT), StrokeKind::Outside);
            let active = matches!(self.drag, Drag::TextWidth { .. });
            let hc = if active { Color32::WHITE } else { ACCENT };
            let hh = 18.0f32.min(box_rect.height());
            p.rect_filled(
                Rect::from_center_size(pos2(text_rect.max.x + 6.0, text_rect.center().y), vec2(4.0, hh)),
                2.0,
                hc,
            );
            if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::Enter)) {
                commit = true;
            }
        }
        if commit {
            self.commit_text(ctx);
        }
    }

    fn paint_bars(&self, p: &Painter, pos: Option<Pos2>, sel: Rect) {
        let (bar, btns, below) = self.layout_bar(sel);
        p.rect_filled(bar.translate(vec2(0.0, 1.5)), 3.0, Color32::from_black_alpha(40));
        p.rect_filled(bar, 3.0, BAR_BG);
        p.rect_stroke(bar, 3.0, Stroke::new(1.0, BAR_LINE), StrokeKind::Inside);
        for (r, b) in &btns {
            if *b == Btn::Sep {
                let x = r.center().x;
                p.line_segment([pos2(x, r.min.y + 6.0), pos2(x, r.max.y - 6.0)], Stroke::new(1.0, BAR_LINE));
                continue;
            }
            let active = matches!(b, Btn::Tool(t) if Some(*t) == self.tool);
            let enabled = match b {
                Btn::Undo => !self.undo.is_empty(),
                Btn::Redo => !self.redo.is_empty(),
                _ => true,
            };
            let hovered = enabled && pos.map(|q| r.contains(q)).unwrap_or(false);
            if hovered || active {
                p.rect_filled(r.shrink(1.0), 3.0, if active { Color32::from_rgb(220, 235, 250) } else { Color32::from_rgb(232, 232, 232) });
            }
            let col = if !enabled { ICON_OFF } else if active { ACCENT } else { ICON };
            draw_icon(p, *b, r.center(), col);
        }

        if let Some((rect, items)) = self.layout_sub(bar, below) {
            p.rect_filled(rect.translate(vec2(0.0, 1.5)), 3.0, Color32::from_black_alpha(40));
            p.rect_filled(rect, 3.0, BAR_BG);
            p.rect_stroke(rect, 3.0, Stroke::new(1.0, BAR_LINE), StrokeKind::Inside);
            for (r, s) in items {
                let c = r.center();
                match s {
                    Sub::Sep => {
                        p.line_segment([pos2(c.x, r.min.y + 5.0), pos2(c.x, r.max.y - 5.0)], Stroke::new(1.0, BAR_LINE));
                    }
                    Sub::Kind(k) => {
                        let on = match self.tool {
                            Some(Tool::Shape) => (k == 1) == self.shape_ellipse,
                            _ => (k == 1) == self.arrow_line,
                        };
                        if on {
                            p.rect_filled(r.shrink(1.0), 3.0, Color32::from_rgb(220, 235, 250));
                        }
                        let col = if on { ACCENT } else { ICON };
                        let st = Stroke::new(1.6, col);
                        if self.tool == Some(Tool::Shape) {
                            if k == 0 {
                                p.rect_stroke(Rect::from_center_size(c, vec2(14.0, 10.0)), 1.0, st, StrokeKind::Middle);
                            } else {
                                p.add(Shape::ellipse_stroke(c, vec2(7.0, 5.0), st));
                            }
                        } else if k == 0 {
                            draw_arrow(p, c - vec2(7.0, -5.0), c + vec2(7.0, -5.0), col, 1.6, true);
                        } else {
                            p.line_segment([c - vec2(7.0, -5.0), c + vec2(7.0, -5.0)], st);
                        }
                    }
                    Sub::Width(i) => {
                        let on = if self.tool == Some(Tool::Text) { i == self.size_idx } else { i == self.width_idx };
                        if on {
                            p.rect_filled(r.shrink(1.0), 3.0, Color32::from_rgb(220, 235, 250));
                        }
                        if self.tool == Some(Tool::Text) {
                            let f = [11.0, 15.0, 19.0][i];
                            p.text(c, Align2::CENTER_CENTER, "A", FontId::proportional(f), if on { ACCENT } else { ICON });
                        } else {
                            p.circle_filled(c, 2.0 + i as f32 * 1.6, if on { ACCENT } else { ICON });
                        }
                    }
                    Sub::Color(i) => {
                        let on = i == self.color_idx;
                        p.circle_filled(c, 8.0, PALETTE[i]);
                        p.circle_stroke(c, 8.0, Stroke::new(1.0, Color32::from_gray(150)));
                        if on {
                            p.circle_stroke(c, 11.0, Stroke::new(1.8, ACCENT));
                        }
                    }
                }
            }
        }
    }

    fn paint_magnifier(&self, p: &Painter, q: Pos2) {
        const N: i32 = 15; // source pixels shown
        const Z: f32 = 8.0; // zoom
        let side = N as f32 * Z;
        let box_h = side + 44.0;
        let mut origin = q + vec2(22.0, 22.0);
        if origin.x + side + 2.0 > self.screen.max.x {
            origin.x = q.x - 22.0 - side;
        }
        if origin.y + box_h > self.screen.max.y {
            origin.y = q.y - 22.0 - box_h;
        }
        let (px, py) = self.pixel_at(q);
        let (w, h) = (self.frame.width as f32, self.frame.height as f32);
        let half = N / 2;
        let uv = Rect::from_min_max(
            pos2((px - half) as f32 / w, (py - half) as f32 / h),
            pos2((px + half + 1) as f32 / w, (py + half + 1) as f32 / h),
        );
        let img = Rect::from_min_size(origin, vec2(side, side));
        p.rect_filled(img.expand(2.0), 0.0, Color32::BLACK);
        p.image(self.tex.id(), img, uv, Color32::WHITE);
        let cell = Rect::from_center_size(img.center(), vec2(Z, Z));
        p.rect_stroke(cell, 0.0, Stroke::new(1.0, Color32::WHITE), StrokeKind::Outside);
        p.rect_stroke(cell.expand(1.0), 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Outside);
        let info = Rect::from_min_size(pos2(origin.x, origin.y + side), vec2(side, 44.0));
        p.rect_filled(info.expand2(vec2(2.0, 0.0)), 0.0, Color32::from_black_alpha(215));
        let [r, g, b] = self.rgb_at(q);
        let font = FontId::monospace(12.0);
        p.text(info.min + vec2(6.0, 4.0), Align2::LEFT_TOP, format!("({px}, {py})"), font.clone(), Color32::WHITE);
        p.text(info.min + vec2(6.0, 18.0), Align2::LEFT_TOP, format!("RGB {r},{g},{b}"), font.clone(), Color32::WHITE);
        p.text(info.min + vec2(6.0, 31.0), Align2::LEFT_TOP, "C: copy #hex", FontId::proportional(10.0), Color32::from_gray(170));
    }

    // ---- export -----------------------------------------------------------

    /// The selection as RGBA pixels with annotations burned in, rendered without touching the screen.
    fn render_selection(&self) -> Option<(u32, u32, Vec<u8>)> {
        let sel = self.sel?;
        let (fw, fh) = (self.frame.width as i32, self.frame.height as i32);
        let k = self.frame.width as f32 / self.screen.width().max(1.0);
        let px = |v: f32, o: f32, max: i32| (((v - o) * k).round() as i32).clamp(0, max);
        let (x0, y0) = (px(sel.min.x, self.screen.min.x, fw), px(sel.min.y, self.screen.min.y, fh));
        let (x1, y1) = (px(sel.max.x, self.screen.min.x, fw), px(sel.max.y, self.screen.min.y, fh));
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
        let mut out = Vec::with_capacity(w * h * 4);
        for y in y0 as usize..y1 as usize {
            let row = (y * fw as usize + x0 as usize) * 4;
            for px in self.frame.rgba[row..row + w * 4].chunks_exact(4) {
                out.extend_from_slice(&[px[0], px[1], px[2], 255]);
            }
        }
        if !self.shapes.is_empty() {
            self.burn_annots(&mut out, w, h, k, (x0 as f32, y0 as f32), sel);
        }
        Some((w as u32, h as u32, out))
    }

    /// Draw the annotations into `out` (the cropped selection) via a headless egui context.
    fn burn_annots(&self, out: &mut [u8], w: usize, h: usize, k: f32, offset_px: (f32, f32), sel: Rect) {
        let ctx = Context::default();
        ctx.add_font_provider(Arc::new(egui_system_font_provider::SystemFontProvider::new()));
        ctx.set_pixels_per_point(k);

        let has_mosaic = self.shapes.iter().any(|s| matches!(s, Annot::Mosaic { .. }));
        let mosaic = has_mosaic.then(|| ctx.load_texture("mosaic", self.mosaic_image(), TextureOptions::NEAREST));
        let mosaic_id = mosaic.as_ref().map(|t| t.id());

        let mut texs = raster::Textures::default();
        let mut shapes = Vec::new();
        // Two passes: the first one lets egui load fonts (system CJK fallbacks) on demand.
        for _ in 0..2 {
            let mut input = egui::RawInput { screen_rect: Some(self.screen), ..Default::default() };
            input.viewports.entry(egui::ViewportId::ROOT).or_default().native_pixels_per_point = Some(k);
            let mut full = ctx.run_ui(input, |ui| {
                let p = ui
                    .ctx()
                    .layer_painter(LayerId::new(Order::Foreground, Id::new("cutline_export")))
                    .with_clip_rect(sel);
                for s in &self.shapes {
                    paint_annot(&p, s, mosaic_id, self.screen);
                }
            });
            texs.apply(&full.textures_delta);
            full.textures_delta.clear();
            shapes = std::mem::take(&mut full.shapes);
        }
        let prims = ctx.tessellate(shapes, k);
        raster::draw(out, w, h, k, (self.screen.min.x, self.screen.min.y), offset_px, &prims, &texs);
    }

    fn spawn_pin(&self, w: u32, h: u32, rgba: Vec<u8>) -> Option<std::process::Child> {
        let sel = self.sel?;
        let path = crate::frame::temp_dir().join(format!("pin_{}.png", chrono::Local::now().format("%H%M%S%3f")));
        let im = image::RgbaImage::from_raw(w, h, rgba)?;
        if im.save(&path).is_err() {
            return None;
        }
        let s = self.frame.scale;
        let pos = self.win_pos + sel.min.to_vec2();
        let exe = std::env::current_exe().ok()?;
        {
            std::process::Command::new(exe)
                .arg("--pin")
                .arg(&path)
                .arg(pos.x.to_string())
                .arg(pos.y.to_string())
                .arg((w as f32 / s).to_string())
                .arg((h as f32 / s).to_string())
                .stdout(std::process::Stdio::piped())
                .spawn()
                .ok()
        }
    }
}

/// Hide the window first so it vanishes instantly; tearing down the GL context afterwards would
/// otherwise show as a flash of black/white.
fn close_overlay(ctx: &Context) {
    ctx.send_viewport_cmd(ViewportCommand::Visible(false));
    ctx.send_viewport_cmd(ViewportCommand::Close);
}

impl eframe::App for Overlay {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 1.0]
    }

    #[cfg(any(debug_assertions, feature = "devscript"))]
    fn raw_input_hook(&mut self, ctx: &Context, raw_input: &mut egui::RawInput) {
        self.script_step(ctx, raw_input);
    }

    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.screen = ctx.content_rect();
        if self.frames == 0 {
            crate::dwm::disable_transitions();
            ctx.send_viewport_cmd(ViewportCommand::Focus);
        }
        self.frames += 1;
        if self.frames == 2 {
            ctx.send_viewport_cmd(ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(ViewportCommand::Focus);
        }

        if self.export.is_none() {
            self.handle_input(&ctx);
        }

        let painter = ctx.layer_painter(LayerId::new(Order::Background, Id::new("cutline_bg")));
        painter.image(
            self.tex.id(),
            self.screen,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );

        // The screen looks the same while a save dialog is open; only input is paused.
        if let Some(job) = self.save_job.take() {
            self.begin_save(&ctx, frame, job);
        }
        self.poll_save(&ctx);
        let pos = ctx.input(|i| i.pointer.latest_pos());
        let fg = ctx.layer_painter(LayerId::new(Order::Middle, Id::new("cutline_fg")));
        self.paint_chrome(&ctx, &fg, pos);

        // Keep the first frames flowing so the window settles quickly.
        if self.frames < 3 {
            ctx.request_repaint();
        }
    }
}

// ---- annotation geometry / painting -----------------------------------------

fn annot_translate(a: &mut Annot, d: Vec2) {
    match a {
        Annot::Rect { r, .. } | Annot::Mosaic { r } => *r = r.translate(d),
        Annot::Line { a, b, .. } => {
            *a += d;
            *b += d;
        }
        Annot::Pen { pts, .. } | Annot::Highlight { pts, .. } => {
            for p in pts {
                *p += d;
            }
        }
        Annot::Text { pos, bounds, .. } => {
            *pos += d;
            *bounds = bounds.translate(d);
        }
    }
}

fn dist_seg(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let l2 = ab.length_sq();
    if l2 < 1e-6 {
        return p.distance(a);
    }
    let t = (((p - a).dot(ab)) / l2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

fn annot_hit(a: &Annot, p: Pos2, tol: f32) -> bool {
    match a {
        Annot::Rect { r, w, ellipse, .. } => {
            let t = tol + w * 0.5;
            if *ellipse {
                let c = r.center();
                let rad = r.size() * 0.5;
                if rad.x < 1.0 || rad.y < 1.0 {
                    return false;
                }
                let n = vec2((p.x - c.x) / rad.x, (p.y - c.y) / rad.y).length();
                (n - 1.0).abs() * rad.x.min(rad.y) <= t
            } else {
                let edges = [
                    (r.left_top(), r.right_top()),
                    (r.right_top(), r.right_bottom()),
                    (r.right_bottom(), r.left_bottom()),
                    (r.left_bottom(), r.left_top()),
                ];
                edges.iter().any(|(a, b)| dist_seg(p, *a, *b) <= t)
            }
        }
        Annot::Line { a, b, w, .. } => dist_seg(p, *a, *b) <= tol + w * 0.5,
        Annot::Pen { pts, w, .. } | Annot::Highlight { pts, w, .. } => {
            pts.windows(2).any(|s| dist_seg(p, s[0], s[1]) <= tol + w * 0.5)
        }
        Annot::Mosaic { r } => r.contains(p),
        Annot::Text { bounds, .. } => bounds.expand(3.0).contains(p),
    }
}

fn draw_arrow(p: &Painter, a: Pos2, b: Pos2, c: Color32, w: f32, head: bool) {
    let d = b - a;
    let len = d.length();
    if len < 1.0 {
        return;
    }
    let dir = d / len;
    if !head {
        p.line_segment([a, b], Stroke::new(w, c));
        return;
    }
    let hl = (9.0 + w * 2.6).min(len * 0.9);
    let perp = vec2(-dir.y, dir.x);
    let base = b - dir * hl;
    p.line_segment([a, base + dir * (hl * 0.35)], Stroke::new(w, c));
    p.add(Shape::convex_polygon(vec![b, base + perp * (hl * 0.42), base - perp * (hl * 0.42)], c, Stroke::NONE));
}

fn paint_annot(p: &Painter, a: &Annot, mosaic: Option<egui::TextureId>, screen: Rect) {
    match a {
        Annot::Rect { r, c, w, ellipse } => {
            if *ellipse {
                p.add(Shape::ellipse_stroke(r.center(), r.size() * 0.5, Stroke::new(*w, *c)));
            } else {
                p.rect_stroke(*r, 0.0, Stroke::new(*w, *c), StrokeKind::Middle);
            }
        }
        Annot::Line { a, b, c, w, arrow } => draw_arrow(p, *a, *b, *c, *w, *arrow),
        Annot::Pen { pts, c, w } => {
            if pts.len() == 1 {
                p.circle_filled(pts[0], *w * 0.5, *c);
            } else {
                p.add(Shape::line(pts.clone(), Stroke::new(*w, *c)));
                // round caps
                p.circle_filled(pts[0], *w * 0.5, *c);
                p.circle_filled(*pts.last().unwrap(), *w * 0.5, *c);
            }
        }
        Annot::Highlight { pts, c, w } => {
            let col = Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 110);
            if pts.len() > 1 {
                p.add(Shape::line(pts.clone(), Stroke::new(*w, col)));
            }
        }
        Annot::Mosaic { r } => {
            if let Some(id) = mosaic {
                let uv = Rect::from_min_max(
                    pos2((r.min.x - screen.min.x) / screen.width(), (r.min.y - screen.min.y) / screen.height()),
                    pos2((r.max.x - screen.min.x) / screen.width(), (r.max.y - screen.min.y) / screen.height()),
                );
                p.image(id, *r, uv, Color32::WHITE);
            }
        }
        Annot::Text { pos, text, c, size, wrap, .. } => {
            let galley = p.layout(text.clone(), FontId::proportional(*size), *c, *wrap);
            p.galley(*pos, galley, *c);
        }
    }
}

// ---- toolbar icons -----------------------------------------------------------

fn arc(c: Pos2, r: f32, a0: f32, a1: f32, n: usize) -> Vec<Pos2> {
    (0..=n)
        .map(|i| {
            let t = a0 + (a1 - a0) * i as f32 / n as f32;
            c + vec2(t.cos(), t.sin()) * r
        })
        .collect()
}

fn draw_icon(p: &Painter, b: Btn, c: Pos2, col: Color32) {
    let st = Stroke::new(1.6, col);
    let o = |x: f32, y: f32| c + vec2(x, y);
    match b {
        Btn::Tool(Tool::Shape) => {
            p.rect_stroke(Rect::from_center_size(o(2.0, 2.0), vec2(14.0, 10.0)), 1.5, st, StrokeKind::Middle);
            p.add(Shape::ellipse_stroke(o(-2.0, -2.0), vec2(7.0, 5.0), st));
        }
        Btn::Tool(Tool::Arrow) => {
            p.add(Shape::line(vec![o(-9.0, 6.0), o(-3.0, -2.0), o(2.0, 3.0), o(7.0, -4.0)], st));
            let tip = o(9.0, -7.0);
            p.add(Shape::convex_polygon(vec![tip, o(3.0, -6.0), o(8.0, -1.0)], col, Stroke::NONE));
        }
        Btn::Tool(Tool::Pen) => {
            p.line_segment([o(-4.0, 4.0), o(6.0, -6.0)], Stroke::new(3.6, col));
            p.add(Shape::convex_polygon(vec![o(-9.0, 9.0), o(-7.0, 3.0), o(-3.0, 7.0)], col, Stroke::NONE));
        }
        Btn::Tool(Tool::Highlighter) => {
            p.line_segment([o(-3.0, 3.0), o(6.0, -6.0)], Stroke::new(6.0, col));
            p.line_segment([o(-9.0, 9.0), o(-1.0, 9.0)], Stroke::new(2.0, col));
            p.line_segment([o(-6.0, 6.0), o(-2.0, 6.0)], Stroke::new(2.0, col));
        }
        Btn::Tool(Tool::Mosaic) => {
            for iy in 0..3 {
                for ix in 0..3 {
                    if (ix + iy) % 2 == 0 {
                        p.rect_filled(
                            Rect::from_min_size(o(-8.0 + ix as f32 * 5.5, -8.0 + iy as f32 * 5.5), vec2(5.5, 5.5)),
                            0.0,
                            col,
                        );
                    }
                }
            }
            p.rect_stroke(Rect::from_center_size(c, vec2(16.5, 16.5)), 0.0, Stroke::new(1.0, col), StrokeKind::Middle);
        }
        Btn::Tool(Tool::Text) => {
            p.text(c + vec2(0.0, 0.5), Align2::CENTER_CENTER, "T", FontId::proportional(19.0), col);
        }
        Btn::Tool(Tool::Eraser) => {
            p.add(Shape::closed_line(vec![o(-9.0, 2.0), o(-1.0, -7.0), o(9.0, 2.0), o(1.0, 10.0)], st));
            p.line_segment([o(-5.0, 6.5), o(5.0, -3.0)], Stroke::new(1.2, col));
        }
        Btn::Undo | Btn::Redo => {
            let flip = if b == Btn::Redo { -1.0 } else { 1.0 };
            let pts: Vec<Pos2> = arc(o(0.0, 1.0), 7.0, -0.2, -3.5, 14).into_iter().map(|q| c + vec2((q.x - c.x) * flip, q.y - c.y)).collect();
            let end = *pts.last().unwrap();
            p.add(Shape::line(pts, st));
            p.add(Shape::convex_polygon(
                vec![end + vec2(flip * -0.5, -5.0), end + vec2(flip * -0.5, 5.0), end + vec2(flip * 6.0, 0.5)],
                col,
                Stroke::NONE,
            ));
        }
        Btn::Close => {
            let s = Stroke::new(2.0, col);
            p.line_segment([o(-6.0, -6.0), o(6.0, 6.0)], s);
            p.line_segment([o(-6.0, 6.0), o(6.0, -6.0)], s);
        }
        Btn::Pin => {
            p.add(Shape::convex_polygon(vec![o(-1.0, -9.0), o(9.0, 1.0), o(5.0, 1.5), o(2.0, 5.0), o(-5.0, -2.0), o(-1.5, -5.0)], col, Stroke::NONE));
            p.line_segment([o(-1.0, 3.0), o(-9.0, 11.0)], Stroke::new(1.8, col));
        }
        Btn::Save => {
            p.rect_stroke(Rect::from_center_size(c, vec2(15.0, 15.0)), 1.5, st, StrokeKind::Middle);
            p.rect_stroke(Rect::from_min_size(o(-4.0, -7.5), vec2(8.0, 5.0)), 0.0, Stroke::new(1.3, col), StrokeKind::Middle);
            p.rect_stroke(Rect::from_min_size(o(-5.0, 1.0), vec2(10.0, 6.5)), 0.0, Stroke::new(1.3, col), StrokeKind::Middle);
        }
        Btn::Copy => {
            p.rect_stroke(Rect::from_min_size(o(-4.0, -9.0), vec2(12.0, 12.0)), 1.5, st, StrokeKind::Middle);
            p.rect_stroke(Rect::from_min_size(o(-8.0, -5.0), vec2(12.0, 12.0)), 1.5, st, StrokeKind::Middle);
        }
        Btn::Sep => {}
    }
}
