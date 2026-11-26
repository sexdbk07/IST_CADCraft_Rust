//! The iStCAD shell: iStCAD's window, laid over the CADCraft engine. iStCAD (`iStCAD_1`,
//! `MainWindow.xaml`) is the reference; sizes, colours and order follow it.
//!
//! Rows, top to bottom:
//! 1. Title bar, 40 pt, `#292F38`: logo, Undo/Redo, New/Open/Save/Save As, File/Edit/View; the
//!    product name and version centred; the command search and Minimize/Maximize/Close.
//! 2. Main tool bar, 50 pt, `#30363F` with a `#555B65` border: Text Style / Dim Style / Layer
//!    pickers, the drawing tools, Settings and Preferences.
//! 3. Drawing tabs, 28 pt: the drawing list button, one tab per drawing (dirty dot, name, ×), `+`.
//! 4. The drawing area, with the auto-hide strip of the 20 most-used commands on its left edge and
//!    the Properties palette on the right (Ctrl+1).
//! 5. The command dock, 86 pt: the command line (prompt, typed text, latest status, history
//!    button) over the Model/layout tabs, `+` and PDF; the drafting toggles over the zoom readout,
//!    zoom buttons, zoom slider and GRID. AutoComplete opens above the command line, the command
//!    history above the dock.
//!
//! Every control acts through a registered command (`app.start` / `app.run`), so the automation
//! interface reaches everything the shell does. Typing still goes through CADCraft's command-line
//! keyboard handling ([`crate::cmdline::keyboard`]); only its drawing moves into the dock.

use std::collections::HashMap;

use egui::{Align2, Color32, FontId, Rect, RichText, Sense, Stroke, pos2, vec2};
use serde_json::{Value, json};

use crate::CadApp;
use crate::istcad_icons::{self as glyphs, Glyph};
use crate::theme::Tokens;

/// Product name shown in the title bar and window captions.
pub const PRODUCT: &str = "IST_CADCraft";
/// IST_CADCraft's own version (the CADCraft core version is `CARGO_PKG_VERSION`).
pub const VERSION: &str = "1.0.0";

/// Row heights, in points (iStCAD's grid rows).
const TITLE_H: f32 = 40.0;
const TOOLBAR_H: f32 = 50.0;
const TABS_H: f32 = 28.0;
const DOCK_H: f32 = 86.0;
/// Height of the command history window that opens above the dock.
const HISTORY_H: f32 = 137.0;
/// How many commands the auto-hide strip offers.
const QUICK_COUNT: usize = 20;

/// iStCAD colours that have no theme token of their own.
mod ink {
    use egui::Color32;
    /// Title-bar and file-button icon strokes.
    pub const ICON_BRIGHT: Color32 = Color32::from_rgb(0xed, 0xf0, 0xf4);
    /// Undo/Redo strokes.
    pub const ICON_SOFT: Color32 = Color32::from_rgb(0xd9, 0xdc, 0xe2);
    /// Tool-bar icon strokes and list text.
    pub const ICON: Color32 = Color32::from_rgb(0xe1, 0xe3, 0xe8);
    /// Labels beside the pickers.
    pub const LABEL: Color32 = Color32::from_rgb(0xc6, 0xca, 0xd2);
    /// The command line and search field.
    pub const FIELD: Color32 = Color32::from_rgb(0x3b, 0x41, 0x4a);
    /// The picker boxes.
    pub const COMBO: Color32 = Color32::from_rgb(0x34, 0x3a, 0x43);
    /// Drawing tabs: active / inactive fill and border.
    pub const TAB_ON: Color32 = Color32::from_rgb(78, 85, 95);
    pub const TAB_ON_EDGE: Color32 = Color32::from_rgb(112, 119, 129);
    pub const TAB_OFF: Color32 = Color32::from_rgb(49, 55, 64);
    pub const TAB_OFF_EDGE: Color32 = Color32::from_rgb(77, 84, 94);
    /// The unsaved / saved dot on a drawing tab.
    pub const DIRTY: Color32 = Color32::from_rgb(231, 178, 89);
    pub const CLEAN: Color32 = Color32::from_rgb(190, 196, 205);
    /// Layout tabs: selected / not selected, and their border.
    pub const LAYOUT_ON: Color32 = Color32::from_rgb(86, 93, 103);
    pub const LAYOUT_OFF: Color32 = Color32::from_rgb(65, 71, 80);
    pub const LAYOUT_EDGE: Color32 = Color32::from_rgb(93, 100, 110);
    /// A status toggle that is on: fill, border, text.
    pub const TOGGLE_ON: Color32 = Color32::from_rgb(0xae, 0xb4, 0xbe);
    pub const TOGGLE_ON_EDGE: Color32 = Color32::from_rgb(0xd2, 0xd5, 0xdb);
    pub const TOGGLE_ON_TEXT: Color32 = Color32::from_rgb(0x29, 0x2e, 0x36);
    /// The history window and the AutoComplete list.
    pub const HISTORY_BG: Color32 = Color32::from_rgba_unmultiplied_const(0x18, 0x22, 0x26, 0xf2);
    pub const HISTORY_TEXT: Color32 = Color32::from_rgb(0xdc, 0xe2, 0xea);
    /// AutoComplete (1.0.233 `CommandSuggestionPopup`): `#343A43`, border `#707780`.
    pub const SUGGEST_BG: Color32 = Color32::from_rgb(0x34, 0x3a, 0x43);
    pub const SUGGEST_EDGE: Color32 = Color32::from_rgb(0x70, 0x77, 0x80);
    /// The edge toolbar open (`AutoHideButtons`): `#30363F`, border `#555B65`; its pin `#414750`.
    pub const EDGE_BG: Color32 = Color32::from_rgb(0x30, 0x36, 0x3f);
    pub const EDGE_BORDER: Color32 = Color32::from_rgb(0x55, 0x5b, 0x65);
    pub const PIN_BG: Color32 = Color32::from_rgb(0x41, 0x47, 0x50);
    pub const PIN_BORDER: Color32 = Color32::from_rgb(0x62, 0x69, 0x73);
    pub const LIST_EDGE: Color32 = Color32::from_rgb(0x70, 0x77, 0x80);
    pub const LIST_DESC: Color32 = Color32::from_rgb(0x9f, 0xad, 0xb2);
    pub const LIST_SELECTED: Color32 = Color32::from_rgb(0x5a, 0x60, 0x69);
    /// The drawing-area frame.
    pub const CANVAS_EDGE: Color32 = Color32::from_rgb(0x62, 0x69, 0x73);
}

/// iStCAD's frequent-command catalog (`FrequentCommandCatalog`): command, caption, icon. The
/// auto-hide strip ranks these by use, ties keeping this order.
const QUICK_CATALOG: &[(&str, &str, Glyph)] = &[
    ("line", "Line", Glyph::Line),
    ("pline", "Polyline", Glyph::Polyline),
    ("circle", "Circle", Glyph::Circle),
    ("rectang", "Rectangle", Glyph::Rectangle),
    ("arc", "Arc", Glyph::Arc),
    ("move", "Move", Glyph::Move),
    ("copy", "Copy", Glyph::Copy),
    ("rotate", "Rotate", Glyph::Rotate),
    ("erase", "Erase", Glyph::Erase),
    ("select", "Select", Glyph::Select),
    ("zoom.extents", "Zoom extents", Glyph::ZoomExtents),
    ("zoom.previous", "Zoom previous", Glyph::ZoomPrevious),
    ("pan", "Pan", Glyph::Pan),
    ("u", "Undo", Glyph::Undo),
    ("redo", "Redo", Glyph::Redo),
    ("new", "New drawing", Glyph::New),
    ("open", "Open drawing", Glyph::Open),
    ("qsave", "Save", Glyph::Save),
    ("saveas", "Save as", Glyph::SaveAs),
    ("plot", "Plot / PDF", Glyph::Plot),
    ("layout", "New layout", Glyph::Layout),
    ("regen", "Regenerate", Glyph::Regen),
];

/// Shell state kept by [`CadApp`].
#[derive(Default)]
pub struct Shell {
    /// Draw the iStCAD shell instead of CADCraft's own chrome.
    pub enabled: bool,
    /// The window has no OS title bar: the title bar draws Minimize / Maximize / Close and the
    /// window edges resize it.
    pub custom_frame: bool,
    /// Text in the title bar's command search.
    pub search: String,
    /// How often each command was started this session (ranks the auto-hide strip).
    usage: HashMap<String, u32>,
    /// The auto-hide strip is expanded.
    quick_open: bool,
    /// The edge toolbar is pinned open (its 📌 toggle).
    quick_pinned: bool,
    /// The view height each drawing first showed, the 100 % mark of the zoom readout.
    zoom_ref: HashMap<String, f64>,
    /// The window caption last sent, so it is only sent again when it changes.
    caption: String,
    /// The Properties palette (Ctrl+1).
    pub props: crate::istcad_props::State,
}

impl Shell {
    /// Count one start of the command typed or clicked as `name` (an alias counts for its command).
    pub fn note(&mut self, name: &str) {
        let name = name.trim().trim_start_matches(['_', '.', '\'', '-']);
        if name.is_empty() || name.starts_with("ui.") {
            return;
        }
        let id = cadcraft_engine::cmd::resolve_alias(name);
        if cadcraft_engine::cmd::find_command(&id).is_some() {
            *self.usage.entry(id).or_default() += 1;
        }
    }

    /// The usage counts as saved with the preferences (`None` before anything was used), as
    /// iStCAD keeps them between sessions.
    pub fn usage_json(&self) -> Option<Value> {
        (!self.usage.is_empty()).then(|| {
            let sorted: std::collections::BTreeMap<&String, &u32> = self.usage.iter().collect();
            json!(sorted)
        })
    }

    /// Restore saved usage counts; entries that are not a command and a count are skipped.
    pub fn load_usage(&mut self, v: &Value) {
        let Some(map) = v.as_object() else { return };
        for (id, n) in map.iter().take(1000) {
            if let Some(n) = n.as_u64().and_then(|n| u32::try_from(n).ok())
                && cadcraft_engine::cmd::find_command(id).is_some()
            {
                self.usage.insert(id.clone(), n);
            }
        }
    }

    /// How often `id` was started (saved counts included).
    pub fn uses(&self, id: &str) -> u32 {
        self.usage.get(id).copied().unwrap_or(0)
    }

    /// The auto-hide strip's commands: [`QUICK_CATALOG`] by use, ties in catalog order.
    pub fn ranked(&self) -> Vec<&'static str> {
        let mut items: Vec<(usize, &'static str)> = QUICK_CATALOG.iter().enumerate().map(|(i, c)| (i, c.0)).collect();
        items.sort_by(|a, b| self.uses(b.1).cmp(&self.uses(a.1)).then(a.0.cmp(&b.0)));
        items.into_iter().map(|(_, id)| id).take(QUICK_COUNT).collect()
    }
}

/// UI commands the shell adds: `perf` (iStCAD's PERF report) and `ui.istcad.*`.
pub fn route(app: &mut CadApp, id: &str, params: &Value) -> Option<Result<Value, String>> {
    match id {
        "perf" => Some(Ok(perf(app))),
        "ui.istcad.shell" => {
            app.istcad.enabled = params.get("on").and_then(Value::as_bool).unwrap_or(!app.istcad.enabled);
            Some(Ok(json!({ "shell": app.istcad.enabled })))
        }
        "ui.istcad.quickbar" => Some(Ok(json!({ "commands": app.istcad.ranked() }))),
        // Ctrl+1 and PROPERTIES (PR, PROPS) show or hide iStCAD's Properties palette. With
        // parameters (`{handles}` from automation) PROPERTIES stays the engine's query.
        "ui.istcad.properties" if app.istcad.enabled => Some(Ok(crate::istcad_props::toggle(app, params.get("on").and_then(Value::as_bool)))),
        "ui.panel.properties" | "properties" | "pr" | "props" if app.istcad.enabled && params.as_object().is_none_or(|o| o.is_empty()) => {
            Some(Ok(crate::istcad_props::toggle(app, None)))
        }
        _ => None,
    }
}

/// PERF: entity counts, render timings and the renderer, echoed to the command history.
fn perf(app: &mut CadApp) -> Value {
    let entities = app.session.doc().map(|d| d.model.len()).unwrap_or(0);
    let prims = app.canvas.list.as_ref().map(|l| l.prims.len()).unwrap_or(0);
    let renderer = if app.canvas.gpu.is_some() { "GPU" } else { "CPU" };
    let line = format!(
        "PERF: model-space entities {entities}, display primitives {prims}, frame {:.1} ms, build {:.1} ms, draw {:.1} ms, renderer {renderer}",
        app.frame_ms, app.canvas.build_ms, app.canvas.draw_ms
    );
    app.session.echo(line.clone());
    json!({
        "entities": entities,
        "primitives": prims,
        "frameMs": app.frame_ms,
        "buildMs": app.canvas.build_ms,
        "drawMs": app.canvas.draw_ms,
        "renderer": renderer,
        "message": line,
    })
}

// ---------------------------------------------------------------- shared controls

/// What an [`icon_button`] draws: the glyph, its colour, its stroke width (at a 24-point icon)
/// and its size in points.
#[derive(Clone, Copy)]
struct IconLook {
    glyph: Glyph,
    color: Color32,
    stroke: f32,
    icon: f32,
}

/// A flat icon button in iStCAD's `ToolButton` style: transparent, `#4A515B` on hover or when
/// `selected`, corner radius 3.
fn icon_button(ui: &mut egui::Ui, size: egui::Vec2, look: IconLook, tip: &str, selected: bool) -> egui::Response {
    let t = Tokens::get();
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if selected || resp.hovered() {
        ui.painter().rect_filled(rect, 3.0, t.control_hover);
    }
    glyphs::paint(ui.painter(), Rect::from_center_size(rect.center(), vec2(look.icon, look.icon)), look.glyph, look.color, look.stroke);
    crate::icons::describe_control(ui, &resp, tip, if selected { Some(true) } else { None }, false);
    resp.on_hover_text(crate::i18n::t(tip))
}

fn look(glyph: Glyph, color: Color32, stroke: f32, icon: f32) -> IconLook {
    IconLook { glyph, color, stroke, icon }
}

/// A flat text button (menus, window buttons, zoom buttons).
fn text_button(ui: &mut egui::Ui, size: egui::Vec2, text: &str, font: f32, tip: &str) -> egui::Response {
    let t = Tokens::get();
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, 3.0, t.control_hover);
    }
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, text, FontId::proportional(font), t.text);
    crate::icons::describe_control(ui, &resp, text, None, false);
    resp.on_hover_text(crate::i18n::t(tip))
}

/// A 1-point vertical rule `h` tall, with `after` points of space after it.
fn rule(ui: &mut egui::Ui, h: f32, after: f32) {
    let t = Tokens::get();
    let (r, _) = ui.allocate_exact_size(vec2(1.0, h), Sense::hover());
    ui.painter().rect_filled(r, 0.0, t.border);
    ui.add_space(after);
}

// ---------------------------------------------------------------- title bar

/// Row 1: logo, quick file/edit buttons, menus, title, search and window buttons.
pub fn title_bar(app: &mut CadApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    egui::Panel::top("ist_title").exact_size(TITLE_H).frame(egui::Frame::NONE.fill(t.chrome_dark)).show(ui, |ui| {
        let r = ui.max_rect();
        // The empty bar moves the window (double-click maximizes); later widgets take their own clicks.
        if app.istcad.custom_frame || app.integrated_titlebar {
            craft_window::title_bar::empty_area(ui, r);
        }
        sync_caption(app, ui.ctx());
        // Window buttons and search on the right, then the left controls with the menus up to
        // them; the title goes centred in the gap between (left of centre when the menus are wide).
        let right = Rect::from_min_max(pos2(r.center().x, r.top()), pos2(r.right() - 8.0, r.bottom()));
        let right_used = ui
            .scope_builder(egui::UiBuilder::new().max_rect(right).layout(egui::Layout::right_to_left(egui::Align::Center)), |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                if app.istcad.custom_frame {
                    window_buttons(ui);
                }
                search_box(app, ui, t);
                ui.min_rect()
            })
            .inner;
        // macOS draws its traffic lights at the top left of the window.
        let inset = if app.integrated_titlebar { 84.0 } else { 12.0 };
        let left = Rect::from_min_max(pos2(r.left() + inset, r.top()), pos2(right_used.left() - 12.0, r.bottom()));
        let left_used = ui
            .scope_builder(egui::UiBuilder::new().max_rect(left), |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.horizontal_centered(|ui| title_left_controls(app, ui, t));
                ui.min_rect()
            })
            .inner;
        paint_title(ui, Rect::from_min_max(pos2(left_used.right() + 12.0, r.top()), pos2(right_used.left() - 12.0, r.bottom())), r, t);
    });
}

/// iStCAD 1.0.233's logo, 30 points: an orange tile (`#D5572B` to `#AA3811`, top to bottom) with
/// a light inner frame, and in white the isometric cube with an "i", an "S" and a pale dot.
fn paint_logo(p: &egui::Painter, rect: Rect, _t: Tokens) {
    let s = rect.width() / 256.0;
    let at = |x: f32, y: f32| pos2(rect.left() + x * s, rect.top() + y * s);
    // The gradient tile: rounded, filled in horizontal bands from the top colour to the bottom one.
    let tile = Rect::from_min_max(at(8.0, 8.0), at(248.0, 248.0));
    let (top, bottom) = ([0xd5_u8, 0x57, 0x2b], [0xaa_u8, 0x38, 0x11]);
    let bands = 12;
    let clip = p.with_clip_rect(tile);
    for i in 0..bands {
        let f = i as f32 / (bands - 1) as f32;
        let mix = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * f).round() as u8;
        let c = Color32::from_rgb(mix(top[0], bottom[0]), mix(top[1], bottom[1]), mix(top[2], bottom[2]));
        let y0 = tile.top() + tile.height() * i as f32 / bands as f32;
        let y1 = tile.top() + tile.height() * (i + 1) as f32 / bands as f32;
        let band = Rect::from_min_max(pos2(tile.left(), y0), pos2(tile.right(), y1 + 0.5));
        // Each band is the tile's rounded shape cut to the band, so the corners stay round.
        clip.with_clip_rect(band).rect_filled(tile, 58.0 * s, c);
    }
    p.rect_stroke(Rect::from_min_max(at(24.0, 24.0), at(232.0, 232.0)), 44.0 * s, Stroke::new(5.0 * s, Color32::from_white_alpha(0x8c)), egui::StrokeKind::Middle);
    // The mark, scaled 0.82 about the tile's centre.
    let m = |x: f32, y: f32| at(128.0 + (x - 128.0) * 0.82, 128.0 + (y - 128.0) * 0.82);
    let white = |w: f32| Stroke::new(w * 0.82 * s, Color32::WHITE);
    p.add(egui::Shape::closed_line(vec![m(128.0, 34.0), m(208.0, 78.0), m(208.0, 176.0), m(128.0, 222.0), m(48.0, 176.0), m(48.0, 78.0)], white(13.0)));
    p.line(vec![m(48.0, 78.0), m(128.0, 126.0), m(208.0, 78.0)], white(13.0));
    p.line_segment([m(128.0, 126.0), m(128.0, 222.0)], white(13.0));
    p.line_segment([m(88.0, 142.0), m(88.0, 184.0)], white(14.0));
    for (a, b, c, d) in [((184.0, 130.0), (158.0, 124.0), (148.0, 146.0), (167.0, 156.0)), ((167.0, 156.0), (186.0, 166.0), (178.0, 188.0), (151.0, 184.0))] {
        p.add(egui::epaint::CubicBezierShape::from_points_stroke(
            [m(a.0, a.1), m(b.0, b.1), m(c.0, c.1), m(d.0, d.1)],
            false,
            Color32::TRANSPARENT,
            white(12.0),
        ));
    }
    p.circle_filled(m(88.0, 116.0), 11.0 * 0.82 * s, Color32::from_rgb(0xff, 0xe2, 0xa6));
}

/// "IST_CADCraft" (14 pt) and " v1.0.0" (9 pt, `#A7AEB8`): centred on the bar `bar`, kept
/// within `gap` (between the menus and the search box); left out when the gap is too narrow.
fn paint_title(ui: &egui::Ui, gap: Rect, bar: Rect, t: Tokens) {
    let p = ui.painter();
    let name = p.layout_no_wrap(PRODUCT.to_string(), FontId::proportional(14.0), t.text);
    let ver = p.layout_no_wrap(format!(" v{VERSION}"), FontId::proportional(9.0), t.canvas_ink);
    let (nw, vw, nh) = (name.size().x, ver.size().x, name.size().y);
    if gap.width() < nw + vw {
        return;
    }
    let r = bar;
    let x = (r.center().x - (nw + vw) / 2.0).clamp(gap.left(), gap.right() - nw - vw);
    let y = r.center().y - nh / 2.0;
    p.galley(pos2(x, y), name, t.text);
    p.galley(pos2(x + nw, y + 4.0), ver, t.canvas_ink);
}

/// The OS window caption names the drawing, as iStCAD's does.
fn sync_caption(app: &mut CadApp, ctx: &egui::Context) {
    let caption = match app.session.state() {
        Ok(st) if !app.ui.start_tab => format!("{} — {PRODUCT}", st.title),
        _ => PRODUCT.to_string(),
    };
    if caption != app.istcad.caption {
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(caption.clone()));
        app.istcad.caption = caption;
    }
}

fn title_left_controls(app: &mut CadApp, ui: &mut egui::Ui, t: Tokens) {
    let (logo, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
    paint_logo(ui.painter(), logo, t);
    ui.add_space(8.0);
    let mut run = None;
    // iStCAD 1.0.233: 28-point buttons, 24-unit paths fitted to 18 points, 1.9-unit `#D9DCE2` strokes.
    let soft = |g| look(g, ink::ICON_SOFT, 1.9, 18.0);
    if icon_button(ui, vec2(28.0, 28.0), soft(Glyph::Undo), "Undo", false).clicked() {
        run = Some("u");
    }
    ui.add_space(2.0);
    if icon_button(ui, vec2(28.0, 28.0), soft(Glyph::Redo), "Redo", false).clicked() {
        run = Some("redo");
    }
    ui.add_space(6.0);
    rule(ui, 22.0, 5.0);
    for (glyph, cmd, tip) in [
        (Glyph::New, "new", "New drawing (Ctrl+N)"),
        (Glyph::Open, "open", "Open drawing (Ctrl+O)"),
        (Glyph::Save, "qsave", "Save drawing (Ctrl+S)"),
        (Glyph::SaveAs, "saveas", "Save drawing as a new file"),
    ] {
        if icon_button(ui, vec2(28.0, 28.0), soft(glyph), tip, false).clicked() {
            run = Some(cmd);
        }
        ui.add_space(2.0);
    }
    ui.add_space(5.0);
    rule(ui, 22.0, 0.0);
    // CADCraft's menus sit here, where iStCAD has its File / Edit / View, folded into seven
    // (File, Edit, View, Draw, Modify, Tools, Help), so there is no second row.
    crate::menus::menu_row(app, ui, |ui, name| {
        let w = ui.painter().layout_no_wrap(name.to_string(), FontId::proportional(13.0), Color32::WHITE).size().x;
        text_button(ui, vec2((w + 16.0).max(36.0), 28.0), name, 13.0, &format!("{name} menu"))
    });
    if let Some(cmd) = run {
        if cmd == "new" {
            app.ui.start_tab = false;
        }
        app.start(cmd);
    }
}

/// `(command, label)` for every command one level under `menu`.
#[cfg(test)]
fn menu_entries(menu: &str) -> Vec<(&'static str, &'static str)> {
    let mut out: Vec<(&'static str, &'static str)> = cadcraft_engine::cmd::command_specs()
        .iter()
        .filter(|c| c.menu.len() == 2 && c.menu.first() == Some(&menu))
        .map(|c| (c.id, c.label))
        .collect();
    for (id, label, path, _) in crate::menus::UI_COMMANDS {
        if path.len() == 2 && path.first() == Some(&menu) {
            out.push((*id, *label));
        }
    }
    out
}

/// Minimize, Maximize/Restore and Close (42 points wide each), for a window without an OS frame.
fn window_buttons(ui: &mut egui::Ui) {
    let maximized = ui.input(|i| i.viewport().maximized).unwrap_or(false);
    if caption_button(ui, 48.0, "×", 27.0, "Close", true).clicked() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
    }
    let (glyph, tip) = if maximized { ("❐", "Restore") } else { ("▢", "Maximize") };
    if caption_button(ui, 46.0, glyph, 17.0, tip, false).clicked() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    }
    if caption_button(ui, 46.0, "─", 12.0, "Minimize", false).clicked() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }
    ui.add_space(8.0);
}

/// iStCAD's caption buttons (`CadCaptionButton`, 46 × 30; Close 48): `#F3F5F8` glyph, `#36414E`
/// under the pointer, `#606B7A` pressed; Close turns `#C42B28` (`#9F211F` pressed).
fn caption_button(ui: &mut egui::Ui, width: f32, glyph: &str, size: f32, tip: &str, close: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 30.0), Sense::click());
    let fill = match (close, resp.is_pointer_button_down_on(), resp.hovered()) {
        (true, true, _) => Some(Color32::from_rgb(0x9f, 0x21, 0x1f)),
        (true, false, true) => Some(Color32::from_rgb(0xc4, 0x2b, 0x28)),
        (false, true, _) => Some(Color32::from_rgb(0x60, 0x6b, 0x7a)),
        (false, false, true) => Some(Color32::from_rgb(0x36, 0x41, 0x4e)),
        _ => None,
    };
    if let Some(f) = fill {
        ui.painter().rect_filled(rect, 0.0, f);
    }
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, glyph, FontId::proportional(size), Color32::from_rgb(0xf3, 0xf5, 0xf8));
    crate::icons::describe_control(ui, &resp, tip, None, false);
    resp.on_hover_text(crate::i18n::t(tip))
}

/// Commands whose name, label or alias contains `query`, best matches first.
pub fn search_commands(query: &str) -> Vec<(&'static str, &'static str)> {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let mut hits: Vec<(u8, &'static str, &'static str)> = cadcraft_engine::cmd::command_specs()
        .iter()
        .filter(|c| !c.id.contains('.') || !c.menu.is_empty())
        .filter_map(|c| {
            let label = c.label.to_ascii_lowercase();
            let rank = if c.id == q || c.aliases.iter().any(|a| *a == q) {
                0
            } else if c.id.starts_with(&q) || label.starts_with(&q) {
                1
            } else if c.id.contains(&q) || label.contains(&q) {
                2
            } else {
                return None;
            };
            Some((rank, c.id, c.label))
        })
        .collect();
    hits.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.len().cmp(&b.1.len())).then_with(|| a.1.cmp(b.1)));
    hits.into_iter().take(12).map(|(_, id, label)| (id, label)).collect()
}

/// The search host (218 × 34, `#3B414A`, radius 5): a magnifier and the query. Typing lists
/// matching commands below it; Enter or a click runs one.
fn search_box(app: &mut CadApp, ui: &mut egui::Ui, t: Tokens) {
    let (host, _) = ui.allocate_exact_size(vec2(218.0, 34.0), Sense::hover());
    ui.painter().rect_filled(host, 5.0, ink::FIELD);
    // iStCAD 1.0.233's "iA" assistant mark (its orange tile) before the field.
    let mark = Rect::from_center_size(pos2(host.left() + 23.0, host.center().y), vec2(22.0, 22.0));
    paint_logo(ui.painter(), mark, t);
    let field = Rect::from_min_max(pos2(host.left() + 43.0, host.top() + 4.0), pos2(host.right() - 10.0, host.bottom() - 4.0));
    let edit = egui::TextEdit::singleline(&mut app.istcad.search)
        .hint_text(crate::tl!("AI Assistant..."))
        .frame(egui::Frame::NONE)
        .text_color(ink::LABEL)
        .desired_width(field.width())
        .align(egui::Align2::LEFT_CENTER);
    let resp = ui.put(field, edit).on_hover_text(crate::tl!("AI Assistant - search commands and tools"));
    let hits = search_commands(&app.istcad.search);
    let mut run = None;
    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        run = hits.first().map(|h| h.0);
    }
    // The list stays up while the pointer is on it, so a click there lands after the field lost focus.
    let keep_id = ui.scope_id().with("ist_search_keep");
    let keep = ui.data(|d| d.get_temp::<bool>(keep_id)).unwrap_or(false);
    let mut over = false;
    if (resp.has_focus() || keep) && !hits.is_empty() {
        let at = host.left_bottom() + vec2(0.0, 4.0);
        egui::Area::new(ui.scope_id().with("ist_search_hits")).order(egui::Order::Foreground).fixed_pos(at).show(ui.ctx(), |ui| {
            list_frame().show(ui, |ui| {
                ui.set_width(host.width().max(300.0));
                for (id, label) in &hits {
                    if suggestion_row(ui, &id.to_ascii_uppercase(), crate::i18n::t(label), false).clicked() {
                        run = Some(*id);
                    }
                }
                over = ui.rect_contains_pointer(ui.min_rect().expand(6.0));
            });
        });
    }
    ui.data_mut(|d| d.insert_temp(keep_id, over && run.is_none()));
    if let Some(id) = run {
        app.istcad.search.clear();
        app.start(id);
    }
}

/// The frame of iStCAD's command lists (`#343A43`, `#707780` border, radius 5, padding 2).
fn list_frame() -> egui::Frame {
    let t = Tokens::get();
    egui::Frame::NONE.fill(t.list_bg).stroke(Stroke::new(1.0, ink::LIST_EDGE)).corner_radius(5).inner_margin(egui::Margin::same(2))
}

/// One row of a command list: the name (130-point column) and its description; `selected` rows
/// are `#5A6069`.
fn suggestion_row(ui: &mut egui::Ui, name: &str, description: &str, selected: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::click());
    if selected || resp.hovered() {
        ui.painter().rect_filled(rect, 2.0, if selected { ink::LIST_SELECTED } else { Tokens::get().control_hover });
    }
    let p = ui.painter().with_clip_rect(rect);
    p.text(pos2(rect.left() + 9.0, rect.center().y), Align2::LEFT_CENTER, name, FontId::proportional(13.0), Color32::WHITE);
    p.text(pos2(rect.left() + 139.0, rect.center().y), Align2::LEFT_CENTER, description, FontId::proportional(12.0), ink::LIST_DESC);
    crate::icons::describe_control(ui, &resp, name, Some(selected), false);
    resp.on_hover_text(description)
}

/// Edge zones that resize a window without an OS frame (the bottom and right edges, as iStCAD's
/// `ResizeBorderThickness="0,0,6,6"`, plus the left edge and the bottom corners).
pub fn resize_edges(app: &CadApp, ctx: &egui::Context) {
    if !app.istcad.custom_frame || ctx.input(|i| i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false)) {
        return;
    }
    let r = ctx.content_rect();
    let Some(p) = ctx.input(|i| i.pointer.hover_pos()) else { return };
    const EDGE: f32 = 6.0;
    let (l, rt, b) = (p.x - r.left() < EDGE, r.right() - p.x < EDGE, r.bottom() - p.y < EDGE);
    use egui::{CursorIcon, ResizeDirection as D};
    let dir = match (l, rt, b) {
        (true, _, true) => Some((D::SouthWest, CursorIcon::ResizeSouthWest)),
        (_, true, true) => Some((D::SouthEast, CursorIcon::ResizeSouthEast)),
        (true, ..) => Some((D::West, CursorIcon::ResizeWest)),
        (_, true, _) => Some((D::East, CursorIcon::ResizeEast)),
        (.., true) => Some((D::South, CursorIcon::ResizeSouth)),
        _ => None,
    };
    if let Some((dir, cursor)) = dir {
        ctx.set_cursor_icon(cursor);
        if ctx.input(|i| i.pointer.primary_pressed()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(dir));
        }
    }
}

// ---------------------------------------------------------------- main tool bar

/// Row 2: style and layer pickers, a rule, the drawing tools, then Settings and Preferences.
pub fn main_toolbar(app: &mut CadApp, ui: &mut egui::Ui) {
    if !app.ui.show_toolbar {
        return;
    }
    let t = Tokens::get();
    let frame = egui::Frame::NONE.fill(t.chrome).stroke(Stroke::new(1.0, t.border)).corner_radius(5).inner_margin(egui::Margin {
        left: 8,
        right: 8,
        top: 0,
        bottom: 1,
    });
    egui::Panel::top("ist_toolbar").exact_size(TOOLBAR_H).frame(frame).show(ui, |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.horizontal_centered(|ui| {
            // iStCAD 1.0.233 dropped the drawing-tool strip: the commands are on the command line
            // and the edge toolbar. The row holds the five style combos.
            pickers(app, ui, t);
            let mut run = None;
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(3.0);
                if icon_button(ui, vec2(38.0, 36.0), look(Glyph::Preferences, ink::ICON, 1.3, 22.0), "Preferences", false).clicked() {
                    run = Some("options");
                }
                if icon_button(ui, vec2(38.0, 36.0), look(Glyph::Settings, ink::ICON, 1.25, 22.0), "Settings", false).clicked() {
                    run = Some("ui.dialog.dsettings");
                }
            });
            if let Some(cmd) = run {
                app.start(cmd);
            }
        });
    });
}

/// One entry of a style combo: what a pick passes on, what it shows, and a colour swatch.
struct Choice {
    key: String,
    label: String,
    swatch: Option<Color32>,
}

impl Choice {
    fn named(name: &str) -> Choice {
        Choice { key: name.to_string(), label: name.to_string(), swatch: None }
    }
}

/// One style combo of the main tool bar.
struct Picker {
    label: &'static str,
    /// Space before the label (iStCAD's margins differ per combo).
    lead: f32,
    id: &'static str,
    choices: Vec<Choice>,
    /// The key shown; `None` when the selected objects differ (the box is blank, as in AutoCAD).
    current: Option<String>,
    /// The command that opens the manager ("Create / Modify...").
    manage: &'static str,
    tip: &'static str,
}

/// What a combo was asked to do.
enum Picked {
    Choice(String),
    Manage(&'static str),
}

/// The colour a swatch shows for `c` (ByLayer and ByBlock: the layer's, white).
fn swatch(c: cadcraft_color::Color, layer: cadcraft_color::Color) -> Color32 {
    let rgb = c.resolve(layer, cadcraft_color::Color::Index(7));
    Color32::from_rgb(rgb.0, rgb.1, rgb.2)
}

/// The value the selected objects share for `f`, `Some(None)` when they differ, `None` with
/// nothing selected (iStCAD's combos then show the current settings).
fn shared<T: PartialEq + Clone>(d: &cadcraft_doc::Drawing, sel: &[cadcraft_doc::Handle], f: impl Fn(&cadcraft_doc::Common) -> T) -> Option<Option<T>> {
    // A very large selection is not scanned (iStCAD stops at 2,000 objects).
    if sel.is_empty() || sel.len() > 2000 {
        return None;
    }
    let mut values = sel.iter().filter_map(|h| d.entity(*h)).map(|e| f(&e.common));
    let first = values.next()?;
    Some(values.all(|v| v == first).then_some(first))
}

/// Text Style, Dim Style, Layer, Color and Line Style, as iStCAD 1.0.233 lays them out. With
/// objects selected, Layer, Color and Line Style show the objects' values (blank when they
/// differ) and a pick changes the objects; otherwise a pick sets the current value. Each list
/// starts with "Create / Modify...", which opens the manager.
fn pickers(app: &mut CadApp, ui: &mut egui::Ui, t: Tokens) {
    use cadcraft_color::Color;
    let Some(d) = app.session.doc().ok() else {
        // No drawing: iStCAD's placeholders, nothing to pick.
        for (label, lead, value) in [("Text Style", 3.0, "Standard"), ("Dim Style", 3.0, "ISO-25"), ("Layer", 3.0, "0"), ("Color", 4.0, "ByLayer"), ("Line Style", 9.0, "ByLayer")] {
            let p = Picker { label, lead, id: label, choices: Vec::new(), current: Some(value.into()), manage: "", tip: "" };
            let _ = combo(ui, &p, t);
        }
        return;
    };
    let sel: Vec<cadcraft_doc::Handle> = app.session.state().map(|st| st.selection.clone()).unwrap_or_default();
    let has_sel = !sel.is_empty();
    let cur_layer = d.header.str("CLAYER", "0");
    let layer_of = |name: &str| d.layer(name).map_or(Color::Index(7), |l| l.color);
    let sel_layer = shared(d, &sel, |c| c.layer.clone());
    let sel_color = shared(d, &sel, |c| c.color);
    let sel_ltype = shared(d, &sel, |c| c.linetype.clone());
    // ByLayer shows the layer in question: the selection's when it has one, else the current one.
    let swatch_layer = match &sel_layer {
        Some(Some(l)) => l.clone(),
        _ => cur_layer.clone(),
    };
    let layer_color = layer_of(&swatch_layer);
    let layer_ltype = d.layer(&swatch_layer).map_or_else(|| "Continuous".to_string(), |l| l.linetype.clone());

    let layers: Vec<Choice> = d.layers.iter().map(|l| Choice { key: l.name.clone(), label: l.name.clone(), swatch: Some(swatch(l.color, l.color)) }).collect();
    let cur_color = match &sel_color {
        Some(c) => *c,
        None => Some(Color::from_aci(d.header.i64("CECOLOR", 256) as i16)),
    };
    let mut colors: Vec<Choice> = [Color::ByLayer, Color::ByBlock]
        .into_iter()
        .chain((1..=7).map(Color::Index))
        .map(|c| Choice { key: c.name(), label: c.name(), swatch: Some(swatch(c, layer_color)) })
        .collect();
    if let Some(c) = cur_color
        && !colors.iter().any(|x| x.key == c.name())
    {
        colors.push(Choice { key: c.name(), label: c.name(), swatch: Some(swatch(c, layer_color)) });
    }
    let mut ltypes = vec![Choice { key: "ByLayer".into(), label: format!("ByLayer ({layer_ltype})"), swatch: None }, Choice::named("ByBlock")];
    ltypes.extend(d.linetypes.iter().filter(|l| !l.name.eq_ignore_ascii_case("ByLayer") && !l.name.eq_ignore_ascii_case("ByBlock")).map(|l| Choice::named(&l.name)));
    let cur_ltype = match &sel_ltype {
        Some(l) => l.clone(),
        None => Some(d.header.str("CELTYPE", "ByLayer")),
    };
    let all = [
        Picker {
            label: "Text Style",
            lead: 3.0,
            id: "ist_textstyle",
            choices: d.text_styles.iter().map(|s| Choice::named(&s.name)).collect(),
            current: Some(d.header.str("TEXTSTYLE", "Standard")),
            manage: "style",
            tip: "Choose text style",
        },
        Picker {
            label: "Dim Style",
            lead: 3.0,
            id: "ist_dimstyle",
            choices: d.dim_styles.iter().map(|s| Choice::named(&s.name)).collect(),
            current: Some(d.header.str("DIMSTYLE", "Standard")),
            manage: "dimstyle",
            tip: "Choose dimension style",
        },
        Picker {
            label: "Layer",
            lead: 3.0,
            id: "ist_layer",
            choices: layers,
            current: match sel_layer {
                Some(l) => l,
                None => Some(cur_layer.clone()),
            },
            manage: "ui.dialog.layers",
            tip: if has_sel { "Layer of the selected objects" } else { "Choose current layer" },
        },
        Picker {
            label: "Color",
            lead: 4.0,
            id: "ist_color",
            choices: colors,
            current: cur_color.map(|c| c.name()),
            manage: "color",
            tip: if has_sel { "Color of the selected objects" } else { "Choose current color" },
        },
        Picker {
            label: "Line Style",
            lead: 9.0,
            id: "ist_linestyle",
            choices: ltypes,
            current: cur_ltype,
            manage: "linetype",
            tip: if has_sel { "Linetype of the selected objects" } else { "Choose current linetype" },
        },
    ];
    let mut pick: Option<(&str, Picked)> = None;
    for picker in &all {
        if let Some(p) = combo(ui, picker, t) {
            pick = Some((picker.id, p));
        }
    }
    match pick {
        Some((_, Picked::Manage(cmd))) => app.start(cmd),
        Some((id, Picked::Choice(key))) => {
            let r = match (id, has_sel) {
                ("ist_textstyle", _) => app.run("style.current", json!({ "name": key })),
                ("ist_dimstyle", _) => app.run("dimstyle.current", json!({ "name": key })),
                ("ist_layer", true) => app.run("properties.set", json!({ "layer": key })),
                ("ist_layer", false) => app.run("layer.current", json!({ "name": key })),
                ("ist_color", true) => app.run("properties.set", json!({ "color": key })),
                ("ist_color", false) => app.run("color", json!({ "color": key })),
                ("ist_linestyle", true) => app.run("properties.set", json!({ "linetype": key })),
                _ => app.run("linetype", json!({ "current": key })),
            };
            if let Err(e) = r {
                app.set_status(e);
            }
        }
        None => {}
    }
}

/// iStCAD's dark combo (`CadDarkCombo`, 148 × 28): face `#56637A`, border `#77869B`, `#263443`
/// under the pointer, the value in white and a small down triangle. Its list (`#394657`, border
/// `#8191A4`) starts with the "Create / Modify..." action, then the choices (`#414D60`, `#263443`
/// under the pointer, `#334B64` for the current one), colours with a 16-point swatch.
fn combo(ui: &mut egui::Ui, picker: &Picker, t: Tokens) -> Option<Picked> {
    const FACE: Color32 = Color32::from_rgb(0x56, 0x63, 0x7a);
    const EDGE: Color32 = Color32::from_rgb(0x77, 0x86, 0x9b);
    const HOVER: Color32 = Color32::from_rgb(0x26, 0x34, 0x43);
    const LIST: Color32 = Color32::from_rgb(0x39, 0x46, 0x57);
    const LIST_EDGE: Color32 = Color32::from_rgb(0x81, 0x91, 0xa4);
    const ITEM: Color32 = Color32::from_rgb(0x41, 0x4d, 0x60);
    const ITEM_ON: Color32 = Color32::from_rgb(0x33, 0x4b, 0x64);
    const ACTION: Color32 = Color32::from_rgb(0x47, 0x5d, 0x76);
    const SWATCH_EDGE: Color32 = Color32::from_rgb(0xd5, 0xe0, 0xec);
    ui.add_space(picker.lead);
    ui.label(RichText::new(crate::i18n::t(picker.label)).size(12.5).color(ink::LABEL));
    ui.add_space(if picker.lead > 3.0 { 5.0 } else { 8.0 });
    let (rect, resp) = ui.allocate_exact_size(vec2(148.0, 28.0), Sense::click());
    let open = egui::Popup::is_id_open(ui.ctx(), egui::Id::unique(picker.id));
    let face = if resp.hovered() || open { HOVER } else { FACE };
    ui.painter().rect(rect, 1.0, face, Stroke::new(1.0, if open { Color32::from_rgb(0xa4, 0xbb, 0xd4) } else { EDGE }), egui::StrokeKind::Inside);
    let p = ui.painter().with_clip_rect(rect.shrink2(vec2(5.0, 1.0)));
    let mut x = rect.left() + 6.0;
    let shown = picker.current.as_deref().and_then(|k| picker.choices.iter().find(|c| c.key.eq_ignore_ascii_case(k)));
    if let Some(sw) = shown.and_then(|c| c.swatch) {
        let r = Rect::from_center_size(pos2(x + 7.0, rect.center().y), vec2(14.0, 14.0));
        p.rect(r, 1.0, sw, Stroke::new(1.0, SWATCH_EDGE), egui::StrokeKind::Inside);
        x += 20.0;
    }
    let text = shown.map(|c| c.label.clone()).or_else(|| picker.current.clone()).unwrap_or_default();
    // The text stops short of the arrow, cut with "…" when it doesn't fit.
    let room = (rect.right() - 22.0 - x).max(0.0);
    let mut job = egui::text::LayoutJob::simple_singleline(text, FontId::proportional(12.5), Color32::WHITE);
    job.wrap = egui::text::TextWrapping::truncate_at_width(room);
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    p.galley(pos2(x, rect.center().y - galley.size().y / 2.0), galley, Color32::WHITE);
    let tri = pos2(rect.right() - 12.0, rect.center().y);
    ui.painter().add(egui::Shape::convex_polygon(
        vec![tri + vec2(-3.5, -2.0), tri + vec2(3.5, -2.0), tri + vec2(0.0, 2.5)],
        Color32::from_rgb(0xf0, 0xf4, 0xf9),
        Stroke::NONE,
    ));
    crate::icons::describe_control(ui, &resp, picker.label, None, false);
    let resp = if picker.tip.is_empty() { resp } else { resp.on_hover_text(crate::i18n::t(picker.tip)) };
    let mut picked = None;
    if picker.choices.is_empty() && picker.manage.is_empty() {
        ui.add_space(8.0);
        return None;
    }
    egui::Popup::menu(&resp).id(egui::Id::unique(picker.id)).frame(egui::Frame::NONE.fill(LIST).stroke(Stroke::new(1.0, LIST_EDGE)).inner_margin(2)).show(|ui| {
        ui.set_min_width(146.0);
        ui.spacing_mut().item_spacing.y = 0.0;
        if !picker.manage.is_empty() {
            let (r, a) = ui.allocate_exact_size(vec2(ui.available_width(), 29.0), Sense::click());
            ui.painter().rect_filled(r, 0.0, if a.hovered() { HOVER } else { ACTION });
            ui.painter().line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(1.0, Color32::from_rgb(0x7c, 0x92, 0xa8)));
            ui.painter().text(r.center(), Align2::CENTER_CENTER, crate::tl!("Create / Modify..."), FontId::proportional(12.5), Color32::WHITE);
            crate::icons::describe_control(ui, &a, "Create / Modify...", None, false);
            if a.on_hover_text(crate::tl!("Open the manager; does not select an item")).clicked() {
                picked = Some(Picked::Manage(picker.manage));
                ui.close();
            }
            ui.add_space(3.0);
        }
        egui::ScrollArea::vertical().max_height(286.0).show(ui, |ui| {
            for c in &picker.choices {
                let current = picker.current.as_deref().is_some_and(|k| k.eq_ignore_ascii_case(&c.key));
                let (r, item) = ui.allocate_exact_size(vec2(ui.available_width(), 25.0), Sense::click());
                let fill = if item.hovered() { HOVER } else if current { ITEM_ON } else { ITEM };
                ui.painter().rect_filled(r, 0.0, fill);
                let mut x = r.left() + 8.0;
                if let Some(sw) = c.swatch {
                    let s = Rect::from_center_size(pos2(x + 8.0, r.center().y), vec2(16.0, 16.0));
                    ui.painter().rect(s, 1.0, sw, Stroke::new(1.0, SWATCH_EDGE), egui::StrokeKind::Inside);
                    x += 22.0;
                }
                ui.painter().with_clip_rect(r).text(pos2(x, r.center().y), Align2::LEFT_CENTER, &c.label, FontId::proportional(12.5), Color32::from_rgb(0xf3, 0xf6, 0xfb));
                crate::icons::describe_control(ui, &item, &c.label, None, false);
                if item.clicked() {
                    if !current {
                        picked = Some(Picked::Choice(c.key.clone()));
                    }
                    ui.close();
                }
            }
        });
    });
    ui.add_space(8.0);
    let _ = t;
    picked
}

// ---------------------------------------------------------------- drawing tabs

/// Row 3: the drawing list button, the drawing tabs and `+`, in iStCAD's tab style.
pub fn drawing_tabs(app: &mut CadApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    egui::Panel::top("ist_tabs").exact_size(TABS_H).frame(egui::Frame::NONE.fill(t.chrome_dark)).show(ui, |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.horizontal(|ui| {
            ui.set_height(TABS_H);
            let list = icon_button(ui, vec2(48.0, 28.0), look(Glyph::Menu, Color32::WHITE, 1.5 * 24.0 / 16.0, 16.0), "Drawing tabs menu", false);
            ui.add_space(4.0);
            let docs: Vec<(usize, String, bool)> = app.session.docs.iter().enumerate().map(|(i, d)| (i, d.title.clone(), d.is_dirty())).collect();
            let active = (!app.ui.start_tab).then_some(app.session.active);
            let mut switch = None;
            let mut close = None;
            let mut start_page = false;
            egui::Popup::menu(&list).show(|ui| {
                ui.set_min_width(220.0);
                if ui.add(egui::Button::selectable(active.is_none(), crate::tl!("Start"))).clicked() {
                    start_page = true;
                    ui.close();
                }
                for (i, title, dirty) in &docs {
                    let text = if *dirty { format!("● {title}") } else { title.clone() };
                    if ui.add(egui::Button::selectable(active == Some(*i), text)).clicked() {
                        switch = Some(*i);
                        ui.close();
                    }
                }
            });
            for (i, title, dirty) in &docs {
                let (clicked, closed) = drawing_tab(app, ui, *i, title, *dirty, active == Some(*i));
                if clicked {
                    switch = Some(*i);
                }
                if closed {
                    close = Some(*i);
                }
            }
            if text_button(ui, vec2(30.0, 28.0), "+", 18.0, "New drawing tab").clicked() {
                app.ui.start_tab = false;
                app.start("new");
            }
            if start_page {
                app.ui.start_tab = true;
            }
            if let Some(i) = switch {
                let _ = app.run("document.switch", json!({ "index": i }));
                app.ui.start_tab = false;
            }
            if let Some(i) = close {
                app.request_close(Some(i));
            }
        });
    });
}

/// The drawing name on a tab: at most 150 points, cut with "…".
fn tab_name(ui: &egui::Ui, title: &str) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(title.to_string(), FontId::proportional(11.0), Color32::WHITE);
    job.wrap = egui::text::TextWrapping { max_width: 150.0, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
    ui.painter().layout_job(job)
}

/// One drawing tab: radius 4,4,0,0, the unsaved/saved dot (7 points), the name (11 points) and
/// ×; inactive tabs sit 1 point lower. Returns (clicked, close clicked).
fn drawing_tab(app: &mut CadApp, ui: &mut egui::Ui, index: usize, title: &str, dirty: bool, active: bool) -> (bool, bool) {
    let name = tab_name(ui, title);
    let w = 8.0 + 7.0 + 7.0 + name.size().x + 6.0 + 21.0 + 5.0;
    let (slot, resp) = ui.allocate_exact_size(vec2(w + 3.0, TABS_H), Sense::click());
    let top = if active { slot.top() } else { slot.top() + 1.0 };
    let rect = Rect::from_min_max(pos2(slot.left(), top), pos2(slot.left() + w, slot.bottom()));
    let (fill, edge) = if active { (ink::TAB_ON, ink::TAB_ON_EDGE) } else { (ink::TAB_OFF, ink::TAB_OFF_EDGE) };
    let p = ui.painter();
    p.rect(rect, egui::CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 }, fill, Stroke::NONE, egui::StrokeKind::Inside);
    // The border runs up the left, across the top and down the right; the bottom is open.
    let s = Stroke::new(1.0, edge);
    p.line(
        vec![
            rect.left_bottom(),
            pos2(rect.left(), rect.top() + 4.0),
            pos2(rect.left() + 4.0, rect.top()),
            pos2(rect.right() - 4.0, rect.top()),
            pos2(rect.right(), rect.top() + 4.0),
            rect.right_bottom(),
        ],
        s,
    );
    p.circle_filled(pos2(rect.left() + 8.0 + 3.5, rect.center().y), 3.5, if dirty { ink::DIRTY } else { ink::CLEAN });
    p.galley(pos2(rect.left() + 22.0, rect.center().y - name.size().y / 2.0), name, Color32::WHITE);
    let close_rect = Rect::from_center_size(pos2(rect.right() - 5.0 - 10.5, rect.center().y), vec2(21.0, 21.0));
    let close = ui.interact(close_rect, ui.scope_id().with(("ist_tab_close", index)), Sense::click());
    if close.hovered() {
        ui.painter().rect_filled(close_rect, 3.0, Tokens::get().control_hover);
    }
    ui.painter().text(close_rect.center(), Align2::CENTER_CENTER, "×", FontId::proportional(13.0), Color32::from_rgb(198, 208, 211));
    crate::icons::describe_control(ui, &close, &format!("Close {title}"), None, false);
    let close = close.on_hover_text(crate::tl!("Close drawing tab"));
    crate::icons::describe_control(ui, &resp, title, Some(active), true);
    let clicked = resp.clicked() && !close.hovered();
    crate::shortcut_menus::attach(app, &resp, |app| crate::shortcut_menus::drawing_tab_entries(app, index));
    (clicked, close.clicked())
}

// ---------------------------------------------------------------- drawing area

/// The drawing area's 1-point `#626973` frame.
pub fn canvas_frame(ctx: &egui::Context, area: Rect) {
    let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::unique("ist_canvas_frame")));
    p.rect_stroke(area, 0.0, Stroke::new(1.0, ink::CANVAS_EDGE), egui::StrokeKind::Inside);
}

/// The auto-hide strip on the drawing area's left edge: a 12 × 58 handle (`#4A515B`, "›") that
/// opens, on hover, a 60-point column of the 20 most-used commands (34 × 32 buttons).
pub fn quick_bar(app: &mut CadApp, ctx: &egui::Context, area: Rect) {
    let ranked = app.istcad.ranked();
    // iStCAD 1.0.233: a 12 × 58 handle; open, a 53-wide strip (`#30363F`, border `#555B65`,
    // padding 3, 5) with the pin toggle (25 high) over the commands.
    let open_h = (ranked.len() as f32 * 34.0 + 25.0 + 4.0 + 14.0).min(area.height() - 8.0).max(58.0);
    let open = app.istcad.quick_open || app.istcad.quick_pinned;
    let (w, h) = if open { (53.0, open_h) } else { (12.0, 58.0) };
    let rect = Rect::from_min_size(pos2(area.left(), area.center().y - h / 2.0), vec2(w, h));
    let mut run = None;
    let mut toggle_pin = false;
    let radius = egui::CornerRadius { nw: 0, sw: 0, ne: 3, se: 3 };
    let pinned = app.istcad.quick_pinned;
    let shell = &app.istcad;
    egui::Area::new(egui::Id::new("ist_quick_bar")).order(egui::Order::Foreground).fixed_pos(rect.min).show(ctx, |ui| {
        let t = Tokens::get();
        let (fill, edge) = if open { (ink::EDGE_BG, ink::EDGE_BORDER) } else { (t.control_hover, ink::CANVAS_EDGE) };
        let frame = egui::Frame::NONE.fill(fill).stroke(Stroke::new(1.0, edge)).corner_radius(radius).inner_margin(if open { egui::Margin::symmetric(3, 5) } else { egui::Margin::ZERO });
        frame.show(ui, |ui| {
            let inner = if open { rect.size() - vec2(6.0, 10.0) } else { rect.size() };
            ui.set_min_size(inner);
            ui.set_max_size(inner);
            if !open {
                let r = ui.max_rect();
                ui.painter().text(r.center(), Align2::CENTER_CENTER, "›", FontId::proportional(19.0), Color32::from_rgb(0xd0, 0xd9, 0xdb));
                let resp = ui.interact(r, ui.scope_id().with("ist_quick_handle"), Sense::hover());
                crate::icons::describe_control(ui, &resp, "Show drawing toolbar", None, false);
                return;
            }
            ui.spacing_mut().item_spacing.y = 4.0;
            if pin_button(ui, pinned).clicked() {
                toggle_pin = true;
            }
            egui::ScrollArea::vertical().max_height(inner.y - 29.0).show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for id in &ranked {
                        let Some((_, caption, glyph)) = QUICK_CATALOG.iter().find(|c| c.0 == *id) else { continue };
                        let tip = format!("{} ({}) — used {} time(s)", crate::i18n::t(caption), id.to_ascii_uppercase(), shell.uses(id));
                        if icon_button(ui, vec2(40.0, 32.0), look(*glyph, Color32::WHITE, 1.4, 20.0), &tip, false).clicked() {
                            run = Some(*id);
                        }
                    }
                });
            });
        });
    });
    if toggle_pin {
        app.istcad.quick_pinned = !app.istcad.quick_pinned;
        let on = app.istcad.quick_pinned;
        app.session.echo(format!("Edge toolbar {}.", if on { "pinned" } else { "unpinned" }));
    }
    // Open while the pointer is over the handle or the open strip (or while pinned).
    let hover = ctx.input(|i| i.pointer.hover_pos()).is_some_and(|p| rect.expand(2.0).contains(p));
    if hover != app.istcad.quick_open {
        app.istcad.quick_open = hover;
        ctx.request_repaint();
    }
    if let Some(id) = run {
        app.start(id);
    }
}

/// The edge toolbar's pin toggle: 25 high, `#414750` with a `#626973` border, a drawing pin
/// standing up while pinned and tilted while not.
fn pin_button(ui: &mut egui::Ui, pinned: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 25.0), Sense::click());
    let t = Tokens::get();
    let fill = if pinned { t.accent.gamma_multiply(0.55) } else if resp.hovered() { t.control_hover } else { ink::PIN_BG };
    let p = ui.painter();
    p.rect(rect, 3.0, fill, Stroke::new(1.0, ink::PIN_BORDER), egui::StrokeKind::Inside);
    let c = rect.center();
    let ink_ = Color32::from_rgb(0xe1, 0xe3, 0xe8);
    // Head, collar and needle; tilted 35° when not pinned.
    let turn = if pinned { 0.0 } else { 35f32.to_radians() };
    let rot = |v: egui::Vec2| c + egui::Vec2::new(v.x * turn.cos() - v.y * turn.sin(), v.x * turn.sin() + v.y * turn.cos());
    p.circle_filled(rot(vec2(0.0, -4.0)), 3.6, ink_);
    p.line_segment([rot(vec2(-4.5, -0.5)), rot(vec2(4.5, -0.5))], Stroke::new(1.6, ink_));
    p.line_segment([rot(vec2(0.0, -0.5)), rot(vec2(0.0, 7.5))], Stroke::new(1.2, ink_));
    let tip = if pinned { "Unpin the edge toolbar" } else { "Pin the edge toolbar open" };
    crate::icons::describe_control(ui, &resp, "Pin / unpin edge toolbar", Some(pinned), false);
    resp.on_hover_text(crate::i18n::t(tip))
}

// ---------------------------------------------------------------- command dock

/// Row 5, 86 points: the command line over the layout tabs (4.35 parts of the width), the
/// drafting toggles over the zoom controls (1.05 parts).
pub fn command_dock(app: &mut CadApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    let frame =
        egui::Frame::NONE.fill(t.chrome).stroke(Stroke::new(1.0, t.cmd_border)).corner_radius(egui::CornerRadius { nw: 5, ne: 5, sw: 0, se: 0 });
    egui::Panel::bottom("ist_dock").exact_size(DOCK_H).frame(frame).show(ui, |ui| {
        let r = ui.max_rect();
        let split = r.left() + r.width() * 4.35 / 5.4;
        let left = Rect::from_min_max(pos2(r.left() + 8.0, r.top() + 5.0), pos2(split, r.bottom() - 2.0));
        let right = Rect::from_min_max(pos2(split + 8.0, r.top() + 5.0), pos2(r.right() - 14.0, r.bottom() - 2.0));
        let bar = Rect::from_min_size(left.min, vec2(left.width(), 40.0));
        let tabs_row = Rect::from_min_max(pos2(left.left() + 6.0, bar.bottom()), left.max);
        let toggles_row = Rect::from_min_size(right.min, vec2(right.width(), 40.0));
        let zoom_row = Rect::from_min_max(pos2(right.left() + 5.0, toggles_row.bottom()), pos2(right.right() - 3.0, right.bottom()));
        if app.ui.show_command_line {
            command_bar(app, ui, bar, r.top(), t);
        }
        row(ui, tabs_row, egui::Layout::left_to_right(egui::Align::Center), |ui| layout_tabs(app, ui));
        row(ui, toggles_row, egui::Layout::right_to_left(egui::Align::Center), |ui| drafting_toggles(app, ui));
        let zoom = row(ui, zoom_row, egui::Layout::right_to_left(egui::Align::Center), |ui| zoom_controls(app, ui, t));
        // iStCAD's three view glyphs go left of the zoom controls, as many as fit beside them.
        let glyphs = Rect::from_min_max(zoom_row.min, pos2((zoom.left() - 6.0).max(zoom_row.left()), zoom_row.bottom()));
        row(ui, glyphs, egui::Layout::left_to_right(egui::Align::Center), |ui| {
            for glyph in ["▧", "▣", "⚙"] {
                if ui.available_width() < 20.0 {
                    break;
                }
                ui.label(RichText::new(glyph).size(13.0).color(t.canvas_ink));
                ui.add_space(12.0);
            }
        });
    });
}

/// Lay out `add` inside `rect` with `layout`.
/// Lay out `add` in `rect`; gives the area it used.
fn row(ui: &mut egui::Ui, rect: Rect, layout: egui::Layout, add: impl FnOnce(&mut egui::Ui)) -> Rect {
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect).layout(layout), |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        add(ui);
        ui.min_rect()
    })
    .inner
}

/// Draws one run of text after another on the command line.
struct Line<'a> {
    painter: egui::Painter,
    font: FontId,
    cy: f32,
    x: f32,
    _ui: std::marker::PhantomData<&'a ()>,
}

impl Line<'_> {
    fn put(&mut self, text: String, color: Color32) -> Rect {
        let g = self.painter.layout_no_wrap(text, self.font.clone(), color);
        let r = Rect::from_min_size(pos2(self.x, self.cy - g.size().y / 2.0), g.size());
        self.painter.galley(r.min, g, color);
        self.x += r.width();
        r
    }
}

/// The command line (`#3B414A`, radius 5): "›", "Command:" or the running command's prompt with
/// its clickable options, the typed text and caret, the latest status (10 points) and the history
/// button. AutoComplete opens above it, the history window above the dock.
fn command_bar(app: &mut CadApp, ui: &mut egui::Ui, bar: Rect, dock_top: f32, t: Tokens) {
    let painter = ui.painter().with_clip_rect(bar);
    painter.rect_filled(bar, 5.0, ink::FIELD);
    let cy = bar.center().y;
    painter.text(pos2(bar.left() + 12.0, cy), Align2::LEFT_CENTER, "›", FontId::proportional(21.0), Color32::WHITE);
    let input_x = bar.left() + 12.0 + 26.0;
    let mut line = Line { painter: painter.clone(), font: FontId::proportional(13.0), cy, x: input_x, _ui: std::marker::PhantomData };
    match app.session.current_prompt() {
        Some(pr) => {
            if let Some(name) = app.session.running.as_ref().filter(|r| r.id != "grip").map(|r| r.id.to_ascii_uppercase()) {
                line.put(format!("{name}  "), t.text_dim);
            }
            let message = if pr.raw { pr.display() } else { crate::i18n::t(&pr.message).to_string() };
            line.put(message, Color32::WHITE);
            if !pr.raw && !pr.keywords.is_empty() {
                let lead = match pr.keyword_lead() {
                    " or [" => crate::tl!(" or ["),
                    lead => lead,
                };
                line.put(lead.to_string(), Color32::WHITE);
                let count = pr.keywords.len();
                for (i, k) in pr.keywords.iter().enumerate() {
                    let r = line.put(k.clone(), t.cmd_keyword);
                    let resp = ui.interact(r.expand2(vec2(1.0, 4.0)), ui.scope_id().with(("ist_kw", i)), Sense::click());
                    if resp.hovered() {
                        painter.rect_filled(r.expand2(vec2(1.0, 3.0)), 2.0, t.cmd_keyword_hover.gamma_multiply(0.6));
                    }
                    crate::icons::describe_control(ui, &resp, k, None, false);
                    if resp.clicked() {
                        let _ = app.session.input(cadcraft_engine::Input::Keyword(k.clone()));
                    }
                    if i + 1 < count {
                        line.put("/".into(), Color32::WHITE);
                    }
                }
                line.put("]".into(), Color32::WHITE);
            }
            if let Some(d) = pr.default.as_ref().filter(|_| !pr.raw) {
                line.put(format!(" <{d}>"), Color32::WHITE);
            }
            if !pr.raw {
                line.put(": ".into(), Color32::WHITE);
            }
        }
        None => {
            line.put(format!("{}  ", crate::tl!("Command:")), Color32::WHITE);
            if app.cmd.buffer.is_empty() && app.cmd.preedit.is_empty() {
                painter.text(pos2(line.x, cy), Align2::LEFT_CENTER, crate::tl!("Type a command..."), line.font.clone(), t.text_faint);
            }
        }
    }
    let typed = line.put(app.cmd.buffer.clone(), Color32::WHITE);
    let mut caret_x = typed.right();
    if !app.cmd.preedit.is_empty() {
        let r = line.put(app.cmd.preedit.clone(), Color32::WHITE);
        painter.line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(1.0, t.accent));
        caret_x = r.right();
    }
    if (ui.input(|i| i.time) * 2.0).floor() as i64 % 2 == 0 {
        painter.line_segment([pos2(caret_x + 1.0, cy - 8.0), pos2(caret_x + 1.0, cy + 8.0)], Stroke::new(1.0, Color32::WHITE));
    }
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
    app.cmd.ime_rect = Some((bar, Rect::from_min_size(pos2(caret_x + 1.0, cy - 8.0), vec2(1.0, 16.0))));
    // Latest status, right-aligned before the history button (at most 260 wide).
    let expand = Rect::from_center_size(pos2(bar.right() - 12.0 - 11.0, cy), vec2(22.0, 22.0));
    let room = (expand.left() - 16.0 - line.x).clamp(0.0, 260.0);
    if room > 20.0 {
        let mut job = egui::text::LayoutJob::simple_singleline(latest_status(app), FontId::proportional(10.0), Color32::WHITE);
        job.wrap = egui::text::TextWrapping { max_width: room, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
        let g = painter.layout_job(job);
        painter.galley(pos2(expand.left() - 8.0 - g.size().x, cy - g.size().y / 2.0), g, Color32::WHITE);
    }
    let resp = ui.interact(expand, ui.scope_id().with("ist_cmd_expand"), Sense::click());
    if resp.hovered() || app.cmd.expanded {
        painter.rect_filled(expand, 3.0, t.control_hover);
    }
    painter.text(expand.center(), Align2::CENTER_CENTER, if app.cmd.expanded { "⌄" } else { "⌃" }, FontId::proportional(14.0), t.text);
    crate::icons::describe_control(ui, &resp, "Command history", Some(app.cmd.expanded), false);
    if resp.on_hover_text(crate::tl!("Expand command history upward")).clicked() {
        app.cmd.expanded = !app.cmd.expanded;
    }
    if app.cmd.expanded {
        history_window(app, ui.ctx(), pos2(bar.left(), dock_top - HISTORY_H), bar.width(), t);
    }
    if !app.session.prompting() {
        autocomplete(app, ui.ctx(), pos2(input_x, bar.top() - 4.0));
    }
}

/// The message the command line's status slot shows: the app's status for five seconds, else the
/// latest command-history line, else "Ready".
fn latest_status(app: &CadApp) -> String {
    app.status
        .as_ref()
        .filter(|(_, at)| crate::now_ms() - at < 5000.0)
        .map(|(m, _)| m.clone())
        .or_else(|| app.session.log.last().cloned())
        .unwrap_or_else(|| crate::tl!("Ready").to_string())
}

/// The command history window above the dock: "Command History" and the log, newest at the bottom.
fn history_window(app: &CadApp, ctx: &egui::Context, at: egui::Pos2, width: f32, t: Tokens) {
    // iStCAD 1.0.233 `CommandExpandedPanel`: 137 high, the canvas's width, `#F2182226`, the
    // history in Consolas 12 `#DCE2EA`, selectable so Ctrl+C copies it.
    let radius = egui::CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 };
    egui::Area::new(egui::Id::new("ist_cmd_history")).order(egui::Order::Foreground).fixed_pos(at).show(ctx, |ui| {
        egui::Frame::NONE
            .fill(ink::HISTORY_BG)
            .stroke(Stroke::new(1.0, ink::CANVAS_EDGE))
            .corner_radius(radius)
            .inner_margin(egui::Margin::symmetric(16, 12))
            .show(ui, |ui| {
                ui.set_width(width - 32.0);
                ui.set_height(HISTORY_H - 24.0);
                ui.label(RichText::new(crate::tl!("Command History")).size(12.0).strong().color(t.text));
                ui.add_space(8.0);
                let mut text = app.session.log.join("\n");
                egui::ScrollArea::vertical().stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
                    let edit = egui::TextEdit::multiline(&mut text)
                        .interactive(true)
                        .frame(egui::Frame::NONE)
                        .font(FontId::monospace(12.0))
                        .text_color(ink::HISTORY_TEXT)
                        .desired_width(f32::INFINITY);
                    let r = ui.add(edit);
                    let _ = r.on_hover_text(crate::tl!("Select text and press Ctrl+C to copy command history"));
                });
            });
    });
}

/// AutoComplete above the command line, as iStCAD 1.0.233's `CommandSuggestionPopup`: `#343A43`,
/// border `#707780`, radius 3, at most 260 × 190, each command's name in 10-point semibold, the
/// first selected (Enter or Space runs it, Tab completes it).
fn autocomplete(app: &mut CadApp, ctx: &egui::Context, bottom_left: egui::Pos2) {
    let sug = crate::cmdline::suggestions_for(&app.session, &app.cmd.buffer);
    if sug.is_empty() {
        return;
    }
    const ROW: f32 = 20.0;
    let h = (sug.len() as f32 * ROW + 6.0).min(190.0);
    let mut run = None;
    egui::Area::new(egui::Id::new("ist_cmd_suggest")).order(egui::Order::Foreground).fixed_pos(bottom_left - vec2(0.0, h + 2.0)).show(ctx, |ui| {
        egui::Frame::NONE.fill(ink::SUGGEST_BG).stroke(Stroke::new(1.0, ink::SUGGEST_EDGE)).corner_radius(3).inner_margin(egui::Margin::same(2)).show(ui, |ui| {
            let width = sug.iter().map(|s| ui.fonts_mut(|f| f.layout_no_wrap(s.display.clone(), FontId::proportional(10.0), Color32::WHITE).size().x)).fold(120.0_f32, f32::max);
            ui.set_width((width + 24.0).min(254.0));
            egui::ScrollArea::vertical().max_height(h - 6.0).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for (i, s) in sug.iter().enumerate() {
                    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::click());
                    if i == 0 || resp.hovered() {
                        ui.painter().rect_filled(rect, 2.0, if i == 0 { ink::LIST_SELECTED } else { Tokens::get().control_hover });
                    }
                    let p = ui.painter().with_clip_rect(rect);
                    let mut job = egui::text::LayoutJob::simple_singleline(s.display.clone(), FontId::proportional(10.5), ink::HISTORY_TEXT);
                    job.wrap = egui::text::TextWrapping::truncate_at_width(rect.width() - 12.0);
                    let galley = ui.fonts_mut(|f| f.layout_job(job));
                    p.galley(pos2(rect.left() + 6.0, rect.center().y - galley.size().y / 2.0), galley, ink::HISTORY_TEXT);
                    crate::icons::describe_control(ui, &resp, &s.display, Some(i == 0), false);
                    if resp.on_hover_text(crate::i18n::t(s.label)).clicked() {
                        run = Some(s.id.to_ascii_lowercase());
                    }
                }
            });
        });
    });
    if let Some(id) = run {
        app.cmd.buffer.clear();
        app.start(&id);
    }
}

/// Model and the layouts (radius 4, `#565D67` when selected), `+` (27 × 27) and PDF (36 × 24).
fn layout_tabs(app: &mut CadApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    let mut tabs = vec!["Model".to_string()];
    let mut current = "Model".to_string();
    if let Ok(st) = app.session.state() {
        if let cadcraft_doc::Space::Paper(n) = &st.space {
            current = n.clone();
        }
        let mut layouts: Vec<_> = st.doc.layouts.iter().collect();
        layouts.sort_by_key(|l| l.tab_order);
        tabs.extend(layouts.iter().map(|l| l.name.clone()));
    }
    let mut switch = None;
    for name in &tabs {
        let model = name == "Model";
        let shown = if model { crate::tl!("Model").to_string() } else { name.clone() };
        let selected = *name == current;
        let font = FontId::proportional(12.0);
        let g = ui.painter().layout_no_wrap(shown.clone(), font, Color32::WHITE);
        let (rect, resp) = ui.allocate_exact_size(vec2((g.size().x + 24.0).max(64.0), 24.0), Sense::click());
        let fill = if selected {
            ink::LAYOUT_ON
        } else if resp.hovered() {
            t.control_hover
        } else {
            ink::LAYOUT_OFF
        };
        ui.painter().rect(rect, 4.0, fill, Stroke::new(1.0, ink::LAYOUT_EDGE), egui::StrokeKind::Inside);
        ui.painter().galley(rect.center() - g.size() / 2.0, g, Color32::WHITE);
        crate::icons::describe_control(ui, &resp, &shown, Some(selected), true);
        let resp = resp.on_hover_text(if model { crate::tl!("Model space").to_string() } else { name.clone() });
        if resp.clicked() && !selected {
            switch = Some(name.clone());
        }
        crate::shortcut_menus::attach(app, &resp, |app| crate::shortcut_menus::layout_tab_entries(app, name));
        ui.add_space(5.0);
    }
    ui.add_space(1.0);
    if text_button(ui, vec2(27.0, 27.0), "+", 16.0, "Create new paper layout").clicked()
        && let Ok(v) = app.run("layout.new", json!({}))
        && let Some(n) = v.get("name").and_then(Value::as_str)
    {
        switch = Some(n.to_string());
    }
    ui.add_space(6.0);
    if text_button(ui, vec2(36.0, 24.0), "PDF", 9.0, "Plot or export to PDF").clicked() {
        app.start("plot");
    }
    if let Some(n) = switch {
        let _ = app.run("layout.set", json!({ "name": n }));
    }
}

/// A status toggle in iStCAD's `StatusToggle` style: `#414750`, radius 3; on: `#AEB4BE` with dark
/// text, so the state never rests on colour hue alone.
fn status_toggle(ui: &mut egui::Ui, label: &str, on: bool, tip: &str, size: egui::Vec2, font: f32) -> egui::Response {
    let t = Tokens::get();
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let (fill, edge, text) = if on {
        (ink::TOGGLE_ON, ink::TOGGLE_ON_EDGE, ink::TOGGLE_ON_TEXT)
    } else if resp.hovered() {
        (t.border, t.border, t.text)
    } else {
        (t.control, t.border, t.text)
    };
    ui.painter().rect(rect, 3.0, fill, Stroke::new(1.0, edge), egui::StrokeKind::Inside);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, FontId::proportional(font), text);
    crate::icons::describe_control(ui, &resp, label, Some(on), false);
    resp.on_hover_text(crate::i18n::t(tip))
}

/// OSNAP ORTHO POLAR OTRACK DYN (18 points high) and Drafting Settings, right-aligned.
fn drafting_toggles(app: &mut CadApp, ui: &mut egui::Ui) {
    use cadcraft_engine::snap::mode;
    let s = app.session.settings.clone();
    let mut toggle = None;
    if icon_button(ui, vec2(38.0, 36.0), look(Glyph::Settings, ink::ICON, 1.2, 22.0), "Drafting settings", false).clicked() {
        toggle = Some("ui.dialog.dsettings");
    }
    // Right to left, so the row reads OSNAP ORTHO POLAR OTRACK DYN.
    for (label, on, cmd, tip) in [
        ("DYN", s.dynmode, "dynmode", "Dynamic input near the pointer"),
        ("OTRACK", s.otrack, "otrack", "Object tracking — click to toggle"),
        ("POLAR", s.polarmode, "polar", "Polar tracking — click to toggle"),
        ("ORTHO", s.orthomode, "ortho", "Ortho mode — click to toggle"),
        ("OSNAP", s.osmode & mode::OFF == 0 && s.osmode != 0, "osnap", "Object Snap — click to toggle; right-click to choose snap modes"),
    ] {
        ui.add_space(5.0);
        let w = ui.painter().layout_no_wrap(label.to_string(), FontId::proportional(8.0), Color32::WHITE).size().x + 8.0;
        let resp = status_toggle(ui, label, on, tip, vec2(w, 18.0), 8.0);
        if resp.clicked() {
            toggle = Some(cmd);
        }
        if cmd == "osnap" {
            resp.context_menu(|ui| {
                if ui.button(crate::tl!("Object snap modes…")).clicked() {
                    toggle = Some("ui.dialog.dsettings");
                    ui.close();
                }
            });
        }
    }
    // Drafting toggles never cancel the running command.
    match toggle {
        Some(c) if c.starts_with("ui.") => app.start(c),
        Some(c) => crate::menus::run_transparent(app, c),
        None => {}
    }
}

/// The zoom readout (100 % = the drawing's first view), −, +, Ext., the 68-point slider and GRID,
/// laid out right to left.
fn zoom_controls(app: &mut CadApp, ui: &mut egui::Ui, t: Tokens) {
    let grid = app.session.settings.gridmode;
    if status_toggle(ui, "GRID", grid, "Show or hide the drawing grid", vec2(38.0, 22.0), 9.0).clicked() {
        crate::menus::run_transparent(app, "grid");
    }
    ui.add_space(4.0);
    let Ok(st) = app.session.state() else { return };
    let height = st.view().height;
    let reference = *app.istcad.zoom_ref.entry(st.title.clone()).or_insert(height);
    let percent = if height > 0.0 { 100.0 * reference / height } else { 100.0 };
    // The slider runs over ±4 decades, logarithmically, 50 = 100 %.
    let mut slider = (50.0 + 12.5 * (percent / 100.0).max(1e-12).log10().clamp(-4.0, 4.0)).clamp(0.0, 100.0) as f32;
    let before = slider;
    let mut factor = None;
    ui.scope(|ui| {
        ui.spacing_mut().slider_width = 68.0;
        if ui.add(egui::Slider::new(&mut slider, 0.0..=100.0).show_value(false)).on_hover_text(crate::tl!("Adjust drawing zoom")).changed()
            && slider != before
        {
            factor = Some(10f64.powf(f64::from(slider - before) / 12.5));
        }
    });
    ui.add_space(4.0);
    let mut run = None;
    if text_button(ui, vec2(45.0, 22.0), "Ext.", 9.0, "Zoom to drawing extents (ZE)").clicked() {
        run = Some("zoom.extents");
    }
    if text_button(ui, vec2(22.0, 22.0), "+", 13.0, "Zoom in").clicked() {
        run = Some("zoom.in");
    }
    if text_button(ui, vec2(22.0, 22.0), "−", 13.0, "Zoom out").clicked() {
        run = Some("zoom.out");
    }
    ui.add_space(6.0);
    ui.label(RichText::new(format!("{percent:.0}%")).size(12.0).color(t.text)).on_hover_text(crate::tl!("Zoom level"));
    if let Some(f) = factor {
        // The canvas rebuilds for the new view in the background when it needs to.
        let _ = app.run("zoom.realtime", json!({ "factor": f }));
    }
    if let Some(cmd) = run {
        crate::menus::run_transparent(app, cmd);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn istcad_quick_bar_ranks_used_commands_first_in_catalog_order() {
        let mut shell = Shell::default();
        let before = shell.ranked();
        assert_eq!(before.len(), QUICK_COUNT);
        assert_eq!(before.first(), Some(&"line"));
        for _ in 0..3 {
            shell.note("RE"); // REGEN's alias counts for REGEN
        }
        shell.note("CO");
        let after = shell.ranked();
        assert_eq!(after.len(), QUICK_COUNT);
        assert_eq!(after.first(), Some(&"regen"));
        assert!(after.iter().position(|c| *c == "copy") < after.iter().position(|c| *c == "line"), "{after:?}");
        // Unused commands keep the catalog order among themselves.
        assert!(after.iter().position(|c| *c == "line") < after.iter().position(|c| *c == "circle"));
    }

    #[test]
    fn istcad_search_finds_commands_by_alias_and_label() {
        assert_eq!(search_commands("rec").first().map(|h| h.0), Some("rectang"));
        assert!(search_commands("circle").iter().any(|h| h.0 == "circle"));
        assert!(search_commands("   ").is_empty());
    }

    #[test]
    fn istcad_menus_list_commands_under_each_menu() {
        for menu in ["File", "Edit", "View"] {
            assert!(!menu_entries(menu).is_empty(), "{menu}");
        }
        assert!(menu_entries("Edit").iter().any(|e| e.0 == "u"));
    }

    #[test]
    fn istcad_quick_catalog_names_real_commands() {
        for (id, ..) in QUICK_CATALOG {
            assert!(cadcraft_engine::cmd::find_command(id).is_some(), "{id}");
        }
    }
}
