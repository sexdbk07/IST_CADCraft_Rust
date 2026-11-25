//! The iStCAD shell: the window layout, tool bars and status dock of iStCAD, laid over the
//! CADCraft engine.
//!
//! Rows, top to bottom (as in iStCAD):
//! 1. Title bar (40 pt): logo, Undo/Redo, New/Open/Save/Save As, File/Edit/View menus; the
//!    product name and version in the middle; command search and the window buttons on the right.
//! 2. Main tool bar (50 pt): Text Style, Dim Style and Layer pickers, then the drawing tools,
//!    then Settings and Preferences.
//! 3. Drawing tabs (CADCraft's file tabs).
//! 4. The drawing area, with an auto-hide strip of the 20 most-used commands on its left edge and
//!    the Properties palette on the right (Ctrl+1).
//! 5. The command dock: layout tabs, New Layout and PDF on the left; OSNAP / ORTHO / POLAR /
//!    OTRACK / DYN, drafting settings, zoom readout, zoom buttons, zoom slider and GRID on the
//!    right.
//!
//! Every button acts through a registered command (`app.start` / `app.run`), so the automation
//! interface reaches everything the shell does.

use std::collections::HashMap;

use egui::{Color32, Rect, RichText, Sense, Stroke, pos2, vec2};
use serde_json::{Value, json};

use crate::CadApp;
use crate::icons::{self, Icon};
use crate::theme::Tokens;

/// Product name shown in the title bar and window captions.
pub const PRODUCT: &str = "IST_CADCraft";
/// IST_CADCraft's own version (the CADCraft core version is `CARGO_PKG_VERSION`).
pub const VERSION: &str = "1.0.0";

/// Heights of the shell rows, in points (iStCAD's grid rows).
const TITLE_H: f32 = 40.0;
const TOOLBAR_H: f32 = 50.0;
const DOCK_H: f32 = 34.0;
/// How many commands the auto-hide strip offers.
const QUICK_COUNT: usize = 20;

/// The commands the auto-hide strip starts with before usage ranks them.
const QUICK_DEFAULT: &[&str] = &[
    "line",
    "pline",
    "circle",
    "arc",
    "rectang",
    "move",
    "copy",
    "erase",
    "trim",
    "extend",
    "offset",
    "rotate",
    "scale",
    "mirror",
    "fillet",
    "hatch",
    "mtext",
    "dimlinear",
    "zoom.extents",
    "select",
];

/// Drawing tools of the main tool bar: icon, command, tooltip.
const TOOLS: &[(Icon, &str, &str)] = &[
    (Icon::SelectObjects, "select", "Select objects (S)"),
    (Icon::Line, "line", "Draw line (L)"),
    (Icon::Circle, "circle", "Draw circle (C)"),
    (Icon::Arc, "arc", "Draw arc (A)"),
    (Icon::Polyline, "pline", "Draw connected line segments (PL)"),
    (Icon::Rectangle, "rectang", "Draw rectangle (REC)"),
    (Icon::Move, "move", "Move selected objects (M)"),
    (Icon::Trim, "trim", "Trim objects (TR)"),
    (Icon::DimLinear, "dimlinear", "Add dimension (DLI)"),
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
    /// The view height each drawing first showed, the 100 % mark of the zoom readout.
    zoom_ref: HashMap<String, f64>,
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

    /// The auto-hide strip's commands: the most used first, filled up from [`QUICK_DEFAULT`].
    pub fn ranked(&self) -> Vec<String> {
        let mut used: Vec<(&String, &u32)> = self.usage.iter().filter(|(id, _)| quick_icon(id).is_some()).collect();
        used.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        let mut out: Vec<String> = used.into_iter().map(|(id, _)| id.clone()).collect();
        for id in QUICK_DEFAULT {
            if out.len() >= QUICK_COUNT {
                break;
            }
            if !out.iter().any(|o| o == id) {
                out.push((*id).to_string());
            }
        }
        out.truncate(QUICK_COUNT);
        out
    }
}

/// The icon the auto-hide strip shows for a command, `None` for commands it doesn't offer.
fn quick_icon(id: &str) -> Option<Icon> {
    Some(match id {
        "line" => Icon::Line,
        "pline" => Icon::Polyline,
        "circle" => Icon::Circle,
        "arc" => Icon::Arc,
        "rectang" => Icon::Rectangle,
        "polygon" => Icon::Polygon,
        "ellipse" => Icon::Ellipse,
        "spline" => Icon::Spline,
        "point" => Icon::Point,
        "xline" => Icon::XLine,
        "hatch" => Icon::Hatch,
        "text" => Icon::Text,
        "mtext" => Icon::MText,
        "move" => Icon::Move,
        "copy" => Icon::Copy,
        "rotate" => Icon::Rotate,
        "scale" => Icon::Scale,
        "mirror" => Icon::Mirror,
        "offset" => Icon::Offset,
        "trim" => Icon::Trim,
        "extend" => Icon::Extend,
        "fillet" => Icon::Fillet,
        "chamfer" => Icon::Chamfer,
        "explode" => Icon::Explode,
        "erase" => Icon::Erase,
        "stretch" => Icon::Stretch,
        "join" => Icon::Join,
        "matchprop" => Icon::MatchProp,
        "dimlinear" => Icon::DimLinear,
        "dimaligned" => Icon::DimAligned,
        "dimradius" => Icon::DimRadius,
        "dimangular" => Icon::DimAngular,
        "insert" => Icon::Insert,
        "zoom.extents" => Icon::ZoomExtents,
        "zoom.window" => Icon::ZoomWindow,
        "select" => Icon::SelectObjects,
        "dist" => Icon::Distance,
        "area" => Icon::Area,
        "list" => Icon::List,
        _ => return None,
    })
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
        paint_title(app, ui, r, t);
        // macOS draws its traffic lights at the top left of the window.
        let inset = if app.integrated_titlebar { 84.0 } else { 12.0 };
        let left = Rect::from_min_max(pos2(r.left() + inset, r.top()), pos2(r.center().x - 90.0, r.bottom()));
        ui.scope_builder(egui::UiBuilder::new().max_rect(left), |ui| {
            ui.horizontal_centered(|ui| title_left_controls(app, ui, t));
        });
        let controls_w = if app.istcad.custom_frame { 3.0 * 42.0 + 8.0 } else { 8.0 };
        let right = Rect::from_min_max(pos2(r.center().x + 90.0, r.top()), pos2(r.right() - 8.0, r.bottom()));
        ui.scope_builder(egui::UiBuilder::new().max_rect(right).layout(egui::Layout::right_to_left(egui::Align::Center)), |ui| {
            if app.istcad.custom_frame {
                window_buttons(ui, t);
            } else {
                ui.add_space(controls_w);
            }
            search_box(app, ui, t);
        });
    });
}

/// The logo mark of iStCAD (an isometric cube in a rounded tile) at `rect`.
fn paint_logo(p: &egui::Painter, rect: Rect, t: Tokens) {
    let s = rect.width() / 32.0;
    let at = |x: f32, y: f32| pos2(rect.left() + x * s, rect.top() + y * s);
    p.rect(rect, 6.0 * s, Color32::from_rgb(0x20, 0x28, 0x32), Stroke::new(1.0, Color32::from_rgb(0x5e, 0x6a, 0x75)), egui::StrokeKind::Inside);
    let teal = Stroke::new(1.65 * s, t.icon_accent);
    let hex = [at(6.0, 8.0), at(16.0, 3.0), at(26.0, 8.0), at(26.0, 21.0), at(16.0, 27.0), at(6.0, 21.0), at(6.0, 8.0)];
    p.line(hex.to_vec(), teal);
    p.line(vec![at(6.0, 8.0), at(16.0, 14.0), at(26.0, 8.0)], teal);
    p.line_segment([at(16.0, 14.0), at(16.0, 27.0)], teal);
    let ink = Stroke::new(1.3 * s, t.text);
    p.line(vec![at(11.0, 11.0), at(16.0, 14.0), at(21.0, 11.0)], ink);
    p.line_segment([at(16.0, 14.0), at(16.0, 21.0)], ink);
    p.circle_filled(at(26.3, 5.7), 1.7 * s, t.icon_point);
}

/// The product name and version, centred, with the current drawing after it.
fn paint_title(app: &CadApp, ui: &egui::Ui, r: Rect, t: Tokens) {
    let p = ui.painter();
    let name = p.layout_no_wrap(PRODUCT.to_string(), egui::FontId::proportional(14.0), t.text);
    let ver = p.layout_no_wrap(format!(" v{VERSION}"), egui::FontId::proportional(9.0), t.text_dim);
    let doc = match app.session.state() {
        Ok(st) if !app.ui.start_tab => format!("  —  {}", st.title),
        _ => String::new(),
    };
    let doc = p.layout_no_wrap(doc, egui::FontId::proportional(12.0), t.text_dim);
    let w = name.size().x + ver.size().x + doc.size().x;
    let x = r.center().x - w / 2.0;
    let y = r.center().y - name.size().y / 2.0;
    let (nw, vw) = (name.size().x, ver.size().x);
    p.galley(pos2(x, y), name, t.text);
    p.galley(pos2(x + nw, y + 3.0), ver, t.text_dim);
    p.galley(pos2(x + nw + vw, y + 1.0), doc, t.text_dim);
}

fn title_left_controls(app: &mut CadApp, ui: &mut egui::Ui, t: Tokens) {
    let (logo, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
    paint_logo(ui.painter(), logo, t);
    ui.add_space(6.0);
    let mut run = None;
    for (icon, cmd, tip) in [(Icon::Undo, "u", "Undo (Ctrl+Z)"), (Icon::Redo, "redo", "Redo (Ctrl+Y)")] {
        if icons::button(ui, icon, 26.0, tip, false).clicked() {
            run = Some(cmd);
        }
    }
    separator(ui, t);
    for (icon, cmd, tip) in [
        (Icon::New, "new", "New drawing (Ctrl+N)"),
        (Icon::Open, "open", "Open drawing (Ctrl+O)"),
        (Icon::Save, "qsave", "Save drawing (Ctrl+S)"),
        (Icon::SaveAs, "saveas", "Save drawing as a new file"),
    ] {
        if icons::button(ui, icon, 29.0, tip, false).clicked() {
            run = Some(cmd);
        }
    }
    separator(ui, t);
    for menu in ["File", "Edit", "View"] {
        menu_button(app, ui, menu);
    }
    if let Some(cmd) = run {
        if cmd == "new" {
            app.ui.start_tab = false;
        }
        app.start(cmd);
    }
}

fn separator(ui: &mut egui::Ui, t: Tokens) {
    let (r, _) = ui.allocate_exact_size(vec2(9.0, 22.0), Sense::hover());
    ui.painter().vline(r.center().x, r.y_range(), Stroke::new(1.0, t.border));
}

/// One of the File / Edit / View menus: the registry's commands placed directly under that menu,
/// in registry order, plus the interface's own commands placed there.
fn menu_button(app: &mut CadApp, ui: &mut egui::Ui, menu: &str) {
    let resp = ui.add(egui::Button::new(RichText::new(crate::i18n::t(menu)).size(13.0)).frame(false).min_size(vec2(52.0, 28.0)));
    let mut chosen = None;
    egui::Popup::menu(&resp).show(|ui| {
        ui.set_min_width(220.0);
        for (id, label) in menu_entries(menu) {
            if ui.button(crate::i18n::t(label)).clicked() {
                chosen = Some(id);
                ui.close();
            }
        }
    });
    if let Some(id) = chosen {
        app.start(id);
    }
}

/// `(command, label)` for every command one level under `menu`.
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

/// Minimize, Maximize/Restore and Close, for a window without an OS title bar.
fn window_buttons(ui: &mut egui::Ui, t: Tokens) {
    let maximized = ui.input(|i| i.viewport().maximized).unwrap_or(false);
    let button = |ui: &mut egui::Ui, text: &str, tip: &str, size: f32| {
        ui.add(egui::Button::new(RichText::new(text).size(size).color(t.text)).frame(false).min_size(vec2(42.0, 32.0)))
            .on_hover_text(crate::i18n::t(tip))
    };
    if button(ui, "×", "Close", 22.0).clicked() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
    }
    if button(ui, if maximized { "❐" } else { "□" }, if maximized { "Restore" } else { "Maximize" }, 16.0).clicked() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    }
    if button(ui, "—", "Minimize", 14.0).clicked() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }
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

/// The title bar's command search: typing lists matching commands; Enter or a click runs one.
fn search_box(app: &mut CadApp, ui: &mut egui::Ui, t: Tokens) {
    let edit = egui::TextEdit::singleline(&mut app.istcad.search).hint_text(format!("⌕ {}", crate::tl!("Search..."))).desired_width(200.0);
    let resp = ui.add(edit).on_hover_text(crate::tl!("Search commands and tools"));
    let hits = search_commands(&app.istcad.search);
    let mut run = None;
    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        run = hits.first().map(|h| h.0);
    }
    // The list stays up while the pointer is on it, so a click there lands after the field lost focus.
    let keep_id = ui.id().with("ist_search_keep");
    let keep = ui.data(|d| d.get_temp::<bool>(keep_id)).unwrap_or(false);
    let mut over = false;
    if (resp.has_focus() || keep) && !hits.is_empty() {
        let at = resp.rect.left_bottom() + vec2(0.0, 4.0);
        egui::Area::new(ui.id().with("ist_search_hits")).order(egui::Order::Foreground).fixed_pos(at).show(ui.ctx(), |ui| {
            egui::Frame::NONE.fill(t.list_bg).stroke(Stroke::new(1.0, t.border)).corner_radius(5).inner_margin(egui::Margin::same(4)).show(
                ui,
                |ui| {
                    ui.set_width(resp.rect.width().max(280.0));
                    for (id, label) in &hits {
                        let text = format!("{}   {}", id.to_ascii_uppercase(), crate::i18n::t(label));
                        if ui.add(egui::Button::selectable(false, text).truncate()).clicked() {
                            run = Some(*id);
                        }
                    }
                    over = ui.rect_contains_pointer(ui.min_rect().expand(6.0));
                },
            );
        });
    }
    ui.data_mut(|d| d.insert_temp(keep_id, over && run.is_none()));
    if let Some(id) = run {
        app.istcad.search.clear();
        app.start(id);
    }
}

/// Edge zones that resize a window without an OS frame.
pub fn resize_edges(app: &CadApp, ctx: &egui::Context) {
    if !app.istcad.custom_frame || ctx.input(|i| i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false)) {
        return;
    }
    let r = ctx.content_rect();
    let Some(p) = ctx.input(|i| i.pointer.hover_pos()) else { return };
    const EDGE: f32 = 5.0;
    let (l, rt, tp, b) = (p.x - r.left() < EDGE, r.right() - p.x < EDGE, p.y - r.top() < EDGE, r.bottom() - p.y < EDGE);
    use egui::{CursorIcon, ResizeDirection as D};
    let dir = match (l, rt, tp, b) {
        (true, _, true, _) => Some((D::NorthWest, CursorIcon::ResizeNorthWest)),
        (_, true, true, _) => Some((D::NorthEast, CursorIcon::ResizeNorthEast)),
        (true, _, _, true) => Some((D::SouthWest, CursorIcon::ResizeSouthWest)),
        (_, true, _, true) => Some((D::SouthEast, CursorIcon::ResizeSouthEast)),
        (true, ..) => Some((D::West, CursorIcon::ResizeWest)),
        (_, true, ..) => Some((D::East, CursorIcon::ResizeEast)),
        (_, _, true, _) => Some((D::North, CursorIcon::ResizeNorth)),
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

/// Row 2: style and layer pickers, drawing tools, settings.
pub fn main_toolbar(app: &mut CadApp, ui: &mut egui::Ui) {
    if !app.ui.show_toolbar {
        return;
    }
    let t = Tokens::get();
    let frame = egui::Frame::NONE.fill(t.chrome).stroke(Stroke::new(1.0, t.border)).corner_radius(5).inner_margin(egui::Margin::symmetric(8, 0));
    egui::Panel::top("ist_toolbar").exact_size(TOOLBAR_H).frame(frame).show(ui, |ui| {
        ui.horizontal_centered(|ui| {
            pickers(app, ui, t);
            separator(ui, t);
            let mut run = None;
            for (icon, cmd, tip) in TOOLS {
                if icons::button(ui, *icon, 32.0, tip, false).clicked() {
                    run = Some(*cmd);
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icons::button(ui, Icon::Gear, 32.0, "Preferences", false).clicked() {
                    run = Some("options");
                }
                if icons::button(ui, Icon::Gear, 32.0, "Settings — drafting settings", false).clicked() {
                    run = Some("ui.dialog.dsettings");
                }
            });
            if let Some(cmd) = run {
                app.start(cmd);
            }
        });
    });
}

/// Text Style, Dim Style and Layer: each lists the drawing's table and makes the pick current.
fn pickers(app: &mut CadApp, ui: &mut egui::Ui, t: Tokens) {
    let Ok(doc) = app.session.doc() else {
        ui.label(RichText::new(crate::tl!("No drawing open")).color(t.text_dim));
        return;
    };
    let text_styles: Vec<String> = doc.text_styles.iter().map(|s| s.name.clone()).collect();
    let dim_styles: Vec<String> = doc.dim_styles.iter().map(|s| s.name.clone()).collect();
    let layers: Vec<String> = doc.layers.iter().map(|l| l.name.clone()).collect();
    let cur_text = doc.header.str("TEXTSTYLE", "Standard");
    let cur_dim = doc.header.str("DIMSTYLE", "Standard");
    let cur_layer = doc.header.str("CLAYER", "0");
    let mut pick: Option<(&str, String)> = None;
    for (label, id, items, current, width, cmd) in [
        ("Text Style", "ist_textstyle", &text_styles, &cur_text, 112.0, "style.current"),
        ("Dim Style", "ist_dimstyle", &dim_styles, &cur_dim, 96.0, "dimstyle.current"),
        ("Layer", "ist_layer", &layers, &cur_layer, 120.0, "layer.current"),
    ] {
        ui.label(RichText::new(crate::i18n::t(label)).color(t.text_dim));
        egui::ComboBox::from_id_salt(id).selected_text(current.as_str()).width(width).show_ui(ui, |ui| {
            for name in items {
                if ui.add(egui::Button::selectable(name == current, name.as_str())).clicked() && name != current {
                    pick = Some((cmd, name.clone()));
                }
            }
        });
        ui.add_space(6.0);
    }
    if let Some((cmd, name)) = pick {
        let _ = app.run(cmd, json!({ "name": name }));
    }
}

// ---------------------------------------------------------------- auto-hide strip

/// The auto-hide strip on the drawing area's left edge: a handle that opens, on hover, a column
/// of the 20 most-used commands.
pub fn quick_bar(app: &mut CadApp, ctx: &egui::Context, area: Rect) {
    let t = Tokens::get();
    let ranked = app.istcad.ranked();
    let size = 34.0;
    let open_h = (ranked.len() as f32 * (size + 2.0) + 10.0).min(area.height() - 16.0).max(58.0);
    let (w, h) = if app.istcad.quick_open { (60.0, open_h) } else { (12.0, 58.0) };
    let rect = Rect::from_min_size(pos2(area.left(), area.center().y - h / 2.0), vec2(w, h));
    let mut run = None;
    egui::Area::new(egui::Id::new("ist_quick_bar")).order(egui::Order::Middle).fixed_pos(rect.min).show(ctx, |ui| {
        let frame = egui::Frame::NONE
            .fill(if app.istcad.quick_open { t.chrome } else { t.control_hover })
            .stroke(Stroke::new(1.0, t.border))
            .corner_radius(egui::CornerRadius { nw: 0, sw: 0, ne: 3, se: 3 });
        frame.show(ui, |ui| {
            ui.set_min_size(rect.size());
            ui.set_max_size(rect.size());
            if !app.istcad.quick_open {
                ui.centered_and_justified(|ui| ui.label(RichText::new("›").size(19.0).color(t.text)));
                return;
            }
            egui::ScrollArea::vertical().max_height(open_h - 6.0).show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);
                    for id in &ranked {
                        let Some(icon) = quick_icon(id) else { continue };
                        let label = cadcraft_engine::cmd::find_command(id).map_or(id.as_str(), |c| c.label);
                        if icons::button(ui, icon, size, label, false).clicked() {
                            run = Some(id.clone());
                        }
                    }
                });
            });
        });
    });
    // Open while the pointer is over the handle or the open strip.
    let hover = ctx.input(|i| i.pointer.hover_pos()).is_some_and(|p| rect.expand(4.0).contains(p));
    if hover != app.istcad.quick_open {
        app.istcad.quick_open = hover;
        ctx.request_repaint();
    }
    if let Some(id) = run {
        app.start(&id);
    }
}

// ---------------------------------------------------------------- command dock

/// Row 5: layout tabs and plotting on the left, drafting toggles and zoom on the right, with the
/// cursor coordinates or the latest message between them.
pub fn command_dock(app: &mut CadApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    let frame =
        egui::Frame::NONE.fill(t.chrome).stroke(Stroke::new(1.0, t.cmd_border)).corner_radius(egui::CornerRadius { nw: 5, ne: 5, sw: 0, se: 0 });
    egui::Panel::bottom("ist_dock").exact_size(DOCK_H).frame(frame.inner_margin(egui::Margin::symmetric(8, 0))).show(ui, |ui| {
        ui.horizontal_centered(|ui| {
            layout_tabs(app, ui, t);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                zoom_controls(app, ui, t);
                ui.add_space(8.0);
                drafting_toggles(app, ui, t);
                ui.add_space(8.0);
                readout(app, ui, t);
            });
        });
    });
}

fn layout_tabs(app: &mut CadApp, ui: &mut egui::Ui, t: Tokens) {
    let Ok(st) = app.session.state() else { return };
    let current = match &st.space {
        cadcraft_doc::Space::Model => "Model".to_string(),
        cadcraft_doc::Space::Paper(n) => n.clone(),
    };
    let mut layouts: Vec<_> = st.doc.layouts.iter().collect();
    layouts.sort_by_key(|l| l.tab_order);
    let mut tabs = vec!["Model".to_string()];
    tabs.extend(layouts.iter().map(|l| l.name.clone()));
    let mut switch = None;
    for name in &tabs {
        let shown = if name == "Model" { crate::tl!("Model").to_string() } else { name.clone() };
        let active = *name == current;
        let text = RichText::new(shown).size(12.0).color(if active { t.text } else { t.text_dim });
        let resp = ui.add(egui::Button::new(text).selected(active).min_size(vec2(64.0, 24.0))).on_hover_text(if name == "Model" {
            crate::tl!("Model space").to_string()
        } else {
            name.clone()
        });
        if resp.clicked() && !active {
            switch = Some(name.clone());
        }
        crate::shortcut_menus::attach(app, &resp, |app| crate::shortcut_menus::layout_tab_entries(app, name));
    }
    if ui.add(egui::Button::new("+").min_size(vec2(26.0, 24.0))).on_hover_text(crate::tl!("Create new paper layout")).clicked()
        && let Ok(v) = app.run("layout.new", json!({}))
        && let Some(n) = v.get("name").and_then(Value::as_str)
    {
        switch = Some(n.to_string());
    }
    if ui
        .add(egui::Button::new(RichText::new("PDF").size(10.0)).min_size(vec2(36.0, 24.0)))
        .on_hover_text(crate::tl!("Plot or export to PDF"))
        .clicked()
    {
        app.start("exportpdf");
    }
    if let Some(n) = switch {
        let _ = app.run("layout.set", json!({ "name": n }));
    }
}

/// A text toggle in iStCAD's status style: light fill and dark text when on.
fn status_toggle(ui: &mut egui::Ui, t: Tokens, label: &str, on: bool, tip: &str) -> egui::Response {
    let (fill, ink) = if on { (Color32::from_rgb(0xae, 0xb4, 0xbe), Color32::from_rgb(0x29, 0x2e, 0x36)) } else { (t.control, t.text) };
    let button =
        egui::Button::new(RichText::new(label).size(10.0).color(ink).strong()).fill(fill).stroke(Stroke::new(1.0, t.border)).corner_radius(3);
    let resp = ui.add(button.min_size(vec2(0.0, 20.0))).on_hover_text(crate::i18n::t(tip));
    icons::describe_control(ui, &resp, label, Some(on), false);
    resp
}

fn drafting_toggles(app: &mut CadApp, ui: &mut egui::Ui, t: Tokens) {
    use cadcraft_engine::snap::mode;
    let s = app.session.settings.clone();
    let mut toggle = None;
    if icons::button(ui, Icon::Gear, 22.0, "Drafting settings", false).clicked() {
        app.start("ui.dialog.dsettings");
    }
    // Right to left, so the row reads OSNAP ORTHO POLAR OTRACK DYN.
    for (label, on, cmd, tip) in [
        ("DYN", s.dynmode, "dynmode", "Dynamic input near the pointer (F12)"),
        ("OTRACK", s.otrack, "otrack", "Object tracking — click to toggle (F11)"),
        ("POLAR", s.polarmode, "polar", "Polar tracking — click to toggle (F10)"),
        ("ORTHO", s.orthomode, "ortho", "Ortho mode — click to toggle (F8)"),
        ("OSNAP", s.osmode & mode::OFF == 0 && s.osmode != 0, "osnap", "Object Snap — click to toggle; right-click for snap modes (F3)"),
    ] {
        let resp = status_toggle(ui, t, label, on, tip);
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

/// The zoom readout (100 % = how the drawing first showed), −, +, Ext., the slider and GRID.
fn zoom_controls(app: &mut CadApp, ui: &mut egui::Ui, t: Tokens) {
    let grid = app.session.settings.gridmode;
    if status_toggle(ui, t, "GRID", grid, "Show or hide the drawing grid (F7)").clicked() {
        crate::menus::run_transparent(app, "grid");
    }
    let Ok(st) = app.session.state() else { return };
    let height = st.view().height;
    let reference = *app.istcad.zoom_ref.entry(st.title.clone()).or_insert(height);
    let percent = if height > 0.0 { 100.0 * reference / height } else { 100.0 };
    // The slider runs over ±4 decades, logarithmically, 50 = 100 %.
    let mut slider = (50.0 + 12.5 * (percent / 100.0).max(1e-12).log10().clamp(-4.0, 4.0)).clamp(0.0, 100.0) as f32;
    let before = slider;
    let mut factor = None;
    if ui.add(egui::Slider::new(&mut slider, 0.0..=100.0).show_value(false)).on_hover_text(crate::tl!("Adjust drawing zoom")).changed()
        && slider != before
    {
        factor = Some(10f64.powf(f64::from(slider - before) / 12.5));
    }
    let mut run = None;
    if ui
        .add(egui::Button::new(RichText::new("Ext.").size(10.0)).min_size(vec2(40.0, 22.0)))
        .on_hover_text(crate::tl!("Zoom to drawing extents (ZE)"))
        .clicked()
    {
        run = Some("zoom.extents");
    }
    if ui.add(egui::Button::new("+").min_size(vec2(22.0, 22.0))).on_hover_text(crate::tl!("Zoom in")).clicked() {
        run = Some("zoom.in");
    }
    if ui.add(egui::Button::new("−").min_size(vec2(22.0, 22.0))).on_hover_text(crate::tl!("Zoom out")).clicked() {
        run = Some("zoom.out");
    }
    ui.label(RichText::new(format!("{percent:.0}%")).size(12.0).color(t.text)).on_hover_text(crate::tl!("Zoom level"));
    if let Some(f) = factor {
        let _ = app.run("zoom.realtime", json!({ "factor": f }));
        app.canvas.list = None;
    }
    if let Some(cmd) = run {
        crate::menus::run_transparent(app, cmd);
        app.canvas.list = None;
    }
}

/// Cursor coordinates, or the latest status message for five seconds.
fn readout(app: &CadApp, ui: &mut egui::Ui, t: Tokens) {
    let message = app.status.as_ref().filter(|(_, at)| crate::now_ms() - at < 5000.0).map(|(m, _)| m.clone());
    let text = message.clone().or_else(|| {
        app.canvas.cursor.map(|c| {
            let c = app.session.ucs_point(c);
            let (lu, lp) = app.session.doc().map(|d| (d.header.i64("LUNITS", 2), d.header.i64("LUPREC", 4))).unwrap_or((2, 4));
            format!("{}, {}", cadcraft_engine::units::format_distance(c.x, lu, lp), cadcraft_engine::units::format_distance(c.y, lu, lp))
        })
    });
    if let Some(text) = text {
        let color = if message.is_some() { t.warn } else { t.text_dim };
        ui.add(egui::Label::new(RichText::new(text).size(11.0).color(color)).truncate());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn istcad_quick_bar_ranks_used_commands_first_and_stays_full() {
        let mut shell = Shell::default();
        let before = shell.ranked();
        assert_eq!(before.len(), QUICK_COUNT);
        for _ in 0..3 {
            shell.note("H"); // HATCH's alias counts for HATCH
        }
        shell.note("CO");
        let after = shell.ranked();
        assert_eq!(after.len(), QUICK_COUNT);
        assert_eq!(after.first().map(String::as_str), Some("hatch"));
        assert!(after.iter().position(|c| c == "copy") < after.iter().position(|c| c == "line"), "{after:?}");
        assert!(after.iter().all(|c| quick_icon(c).is_some()));
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
}
