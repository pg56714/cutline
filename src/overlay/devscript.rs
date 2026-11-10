//! Debug-only scripted input for the overlay, so the whole flow can be tested
//! without moving the real mouse. Enabled by the `CUTLINE_SCRIPT` env var:
//!
//! ```text
//! w 5; m 400 300; d; m 800 600; u; s selected; btn Shape; sub Color 4; ...
//! ```
//!
//! ops: `m x y` move, `d` press, `u` release, `r` right-click, `w n` wait n frames,
//! `s name` save an internal screenshot to `%TEMP%\cutline\<name>.png`,
//! `btn <Shape|Arrow|Pen|Highlighter|Mosaic|Text|Eraser|Undo|Redo|Close|Pin|Save|Copy>`,
//! `sub <Kind|Width|Color> <n>`, `text <string>`, `esc`, `enter`, `ctrlz`, `ctrly`, `close`.

use super::*;
use std::collections::VecDeque;

pub enum Op {
    Move(f32, f32),
    Down,
    Up,
    Right,
    Key(Key, bool),
    Text(String),
    Shot(String),
    Wait(u32),
    Btn(String),
    Sub(String),
    Shift(bool),
    Ctrl(bool),
    Close,
}

pub struct Script {
    q: VecDeque<Op>,
    wait: u32,
    pos: Pos2,
    shift: bool,
    ctrl: bool,
}

fn mods_of(shift: bool, ctrl: bool) -> egui::Modifiers {
    egui::Modifiers { shift, ctrl, command: ctrl, ..egui::Modifiers::NONE }
}

pub fn parse(s: &str) -> Script {
    let mut q = VecDeque::new();
    for part in s.split(';') {
        let t: Vec<&str> = part.split_whitespace().collect();
        let Some(&head) = t.first() else { continue };
        let f = |i: usize| t.get(i).and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0);
        let op = match head {
            "m" => Op::Move(f(1), f(2)),
            "d" => Op::Down,
            "u" => Op::Up,
            "r" => Op::Right,
            "w" => Op::Wait(f(1) as u32),
            "s" => Op::Shot(t.get(1).copied().unwrap_or("shot").to_string()),
            "btn" => Op::Btn(t.get(1).copied().unwrap_or("").to_string()),
            "sub" => Op::Sub(t[1..].join(" ")),
            "text" => Op::Text(t[1..].join(" ")),
            "esc" => Op::Key(Key::Escape, false),
            "enter" => Op::Key(Key::Enter, false),
            "ctrlz" => Op::Key(Key::Z, true),
            "ctrly" => Op::Key(Key::Y, true),
            "ctrls" => Op::Key(Key::S, true),
            "f3" => Op::Key(Key::F3, false),
            "keyc" => Op::Key(Key::C, false),
            "shift" => Op::Shift(t.get(1) == Some(&"on")),
            "dbl" => {
                // two quick clicks, one event per frame
                for op in [Op::Down, Op::Up, Op::Down] {
                    q.push_back(op);
                }
                Op::Up
            }
            "close" => Op::Close,
            _ => continue,
        };
        q.push_back(op);
    }
    Script { q, wait: 6, pos: Pos2::ZERO, shift: false, ctrl: false }
}

impl Overlay {
    pub fn script_step(&mut self, ctx: &Context, raw: &mut egui::RawInput) {
        let Some(sc) = self.script.as_mut() else { return };
        ctx.request_repaint();
        if sc.wait > 0 {
            sc.wait -= 1;
            return;
        }
        let Some(op) = sc.q.pop_front() else { return };
        let mods = mods_of(sc.shift, sc.ctrl);
        match op {
            // Modifiers are tracked by egui through ModifiersChanged events, one frame
            // apart from the key they modify, exactly like a real keyboard.
            Op::Shift(on) => {
                sc.shift = on;
                raw.events.push(egui::Event::ModifiersChanged(mods_of(sc.shift, sc.ctrl)));
            }
            Op::Ctrl(on) => {
                sc.ctrl = on;
                raw.events.push(egui::Event::ModifiersChanged(mods_of(sc.shift, sc.ctrl)));
            }
            Op::Key(key, true) => {
                sc.q.push_front(Op::Ctrl(false));
                sc.q.push_front(Op::Key(key, false));
                sc.q.push_front(Op::Ctrl(true));
            }
            Op::Move(x, y) => {
                sc.pos = pos2(x, y);
                raw.events.push(egui::Event::PointerMoved(sc.pos));
            }
            Op::Down | Op::Up => raw.events.push(egui::Event::PointerButton {
                pos: sc.pos,
                button: egui::PointerButton::Primary,
                pressed: matches!(op, Op::Down),
                modifiers: mods,
            }),
            Op::Right => {
                for pressed in [true, false] {
                    raw.events.push(egui::Event::PointerButton {
                        pos: sc.pos,
                        button: egui::PointerButton::Secondary,
                        pressed,
                        modifiers: mods,
                    });
                }
            }
            Op::Wait(n) => sc.wait = n,
            Op::Key(key, false) => {
                for pressed in [true, false] {
                    raw.events.push(egui::Event::Key {
                        key,
                        physical_key: Some(key),
                        pressed,
                        repeat: false,
                        modifiers: mods,
                    });
                }
            }
            Op::Text(s) => raw.events.push(egui::Event::Text(s)),
            Op::Shot(name) => {
                ctx.send_viewport_cmd(ViewportCommand::Screenshot(ScreenshotCallback::new(move |img| {
                    let (w, h) = (img.size[0] as u32, img.size[1] as u32);
                    let px: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                    if let Some(im) = image::RgbaImage::from_raw(w, h, px) {
                        let _ = im.save(crate::frame::temp_dir().join(format!("{name}.png")));
                    }
                })));
            }
            Op::Close => ctx.send_viewport_cmd(ViewportCommand::Close),
            Op::Btn(name) => {
                let target = self.sel.and_then(|sel| {
                    let (_, btns, _) = self.layout_bar(sel);
                    btns.into_iter().find(|(_, b)| btn_name(*b) == name).map(|(r, _)| r.center())
                });
                self.script_click(target, &name);
            }
            Op::Sub(name) => {
                let target = self.sel.and_then(|sel| {
                    let (bar, _, below) = self.layout_bar(sel);
                    let (_, items) = self.layout_sub(bar, below)?;
                    items.into_iter().find(|(_, s)| sub_name(*s) == name).map(|(r, _)| r.center())
                });
                self.script_click(target, &name);
            }
        }
    }

    /// Expand a symbolic click into move / press / release, one per frame.
    fn script_click(&mut self, target: Option<Pos2>, what: &str) {
        let Some(sc) = self.script.as_mut() else { return };
        match target {
            Some(c) => {
                sc.q.push_front(Op::Up);
                sc.q.push_front(Op::Down);
                sc.q.push_front(Op::Move(c.x, c.y));
            }
            None => eprintln!("[script] no such control: {what}"),
        }
    }
}

fn btn_name(b: Btn) -> String {
    match b {
        Btn::Tool(Tool::Shape) => "Shape",
        Btn::Tool(Tool::Arrow) => "Arrow",
        Btn::Tool(Tool::Pen) => "Pen",
        Btn::Tool(Tool::Highlighter) => "Highlighter",
        Btn::Tool(Tool::Mosaic) => "Mosaic",
        Btn::Tool(Tool::Text) => "Text",
        Btn::Tool(Tool::Eraser) => "Eraser",
        Btn::Undo => "Undo",
        Btn::Redo => "Redo",
        Btn::Close => "Close",
        Btn::Pin => "Pin",
        Btn::Save => "Save",
        Btn::Copy => "Copy",
        Btn::Sep => "-",
    }
    .to_string()
}

fn sub_name(s: Sub) -> String {
    match s {
        Sub::Kind(i) => format!("Kind {i}"),
        Sub::Width(i) => format!("Width {i}"),
        Sub::Color(i) => format!("Color {i}"),
        Sub::Sep => "-".to_string(),
    }
}
