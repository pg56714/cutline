//! A pinned screenshot: borderless, always-on-top window showing one image.
//! Drag to move, wheel to zoom, Ctrl+C to copy, Esc / right-click / double-click to close.

use eframe::egui::{
    self, Color32, ColorImage, Key, Rect, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui,
    ViewportBuilder, ViewportCommand, pos2, vec2,
};

struct Pin {
    tex: TextureHandle,
    rgba: Vec<u8>,
    px: (u32, u32),
    /// Current logical size.
    size: egui::Vec2,
}

pub fn run(args: &[String]) {
    // args: <png> <x> <y> <w> <h>
    if args.len() < 5 {
        return;
    }
    let path = std::path::PathBuf::from(&args[0]);
    let f = |i: usize| args[i].parse::<f32>().unwrap_or(0.0);
    let (x, y, w, h) = (f(1), f(2), f(3).max(16.0), f(4).max(16.0));
    let Ok(img) = image::open(&path) else { return };
    let img = img.to_rgba8();
    let _ = std::fs::remove_file(&path);
    let (pw, ph) = img.dimensions();
    let rgba = img.into_raw();

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("Cutline pin")
            .with_decorations(false)
            .with_always_on_top()
            .with_taskbar(false)
            .with_position(pos2(x, y))
            .with_inner_size(vec2(w, h)),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Cutline pin",
        options,
        Box::new(move |cc| {
            let ci = ColorImage::from_rgba_unmultiplied([pw as usize, ph as usize], &rgba);
            let tex = cc.egui_ctx.load_texture("pin", ci, TextureOptions::LINEAR);
            Ok(Box::new(Pin { tex, rgba, px: (pw, ph), size: vec2(w, h) }))
        }),
    );
}

impl eframe::App for Pin {
    fn clear_color(&self, _v: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 1.0]
    }

    fn ui(&mut self, ui: &mut Ui, _f: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let rect = ctx.content_rect();
        let p = ctx.layer_painter(egui::LayerId::background());
        p.image(self.tex.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        p.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::from_rgb(30, 136, 229)), StrokeKind::Inside);

        let (esc, copy, pressed, secondary, double, scroll) = ctx.input(|i| {
            (
                i.key_pressed(Key::Escape),
                i.events.iter().any(|e| matches!(e, egui::Event::Copy)),
                i.pointer.button_pressed(egui::PointerButton::Primary),
                i.pointer.button_pressed(egui::PointerButton::Secondary),
                i.pointer.button_double_clicked(egui::PointerButton::Primary),
                // Sum the raw wheel events of this frame rather than using the smoothed delta:
                // smoothing spreads one notch over many frames and would compound the zoom
                // step on every one of them.
                i.events
                    .iter()
                    .filter_map(|e| match e {
                        egui::Event::MouseWheel { delta, .. } => Some(delta.y),
                        _ => None,
                    })
                    .sum::<f32>(),
            )
        });
        if esc || secondary || double {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        } else if pressed {
            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
        }
        if copy {
            if let Ok(mut cb) = arboard::Clipboard::new() {
                let _ = cb.set_image(arboard::ImageData {
                    width: self.px.0 as usize,
                    height: self.px.1 as usize,
                    bytes: self.rgba.clone().into(),
                });
            }
        }
        if scroll.abs() > 0.5 {
            let factor = (1.0 + scroll.signum() * 0.08).clamp(0.5, 2.0);
            let ns = (self.size * factor).clamp(vec2(24.0, 24.0), vec2(8000.0, 8000.0));
            if ns != self.size {
                self.size = ns;
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(ns));
            }
        }
    }
}
