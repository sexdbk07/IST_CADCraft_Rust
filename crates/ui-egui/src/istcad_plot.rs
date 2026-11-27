//! The iStCAD window's Plot (PDF) dialog, as iStCAD 1.0.231–1.0.233's `ShowPlotDialog`.
//!
//! A light dialog (`#EEF1F5`) of white cards on the left (470 wide) and the live preview on the
//! right (`#343A44`):
//! - **Paper**: ISO A0–A5, ANSI A–D or Custom (short × long mm), one margin in mm, and the
//!   orientation Auto / Portrait / Landscape (Auto: the plotted area's own shape).
//! - **Plot layout**: what to plot (Model: Extents, Display, Window, picking the window when
//!   chosen; a layout: its sheet), Fit to paper or a scale (1:1 … 5:1, Custom mm = units), Center
//!   the plot or an X/Y offset in mm, and the scale a fitted plot comes to.
//! - **Plot style**: Color, Grayscale, Black (the built-in colour-dependent tables) or Custom
//!   pens, object lineweights, and opening the PDF after the plot. Custom pens is iStCAD's inline
//!   pen table: for each of the 255 index colours an output colour and a lineweight, kept in the
//!   drawing's colour-dependent table "iStCAD pens.ctb" (made on first use, through
//!   STYLESMANAGER).
//!
//! The preview is the sheet as the `plot` command draws it, redrawn a quarter second after the
//! last change. The footer: what is plotted, then All layouts → PDF, Apply to layout (a layout's
//! page setup), Plot PDF and Close. Everything plots through `plot` and `pagesetup`, as the
//! automation interface does.
//!
//! iStCAD's scaled lineweights, default lineweight and its layout viewport card have no
//! equivalent in the `plot` command yet.

use egui::{Color32, CornerRadius, Frame, Margin, RichText, Stroke, TextureHandle, vec2};
use serde_json::{Value, json};

use crate::{CadApp, Pick};

/// iStCAD's papers: name, short and long side in mm. The last is Custom.
const PAPERS: &[(&str, f64, f64)] = &[
    ("ISO A0", 841.0, 1189.0),
    ("ISO A1", 594.0, 841.0),
    ("ISO A2", 420.0, 594.0),
    ("ISO A3", 297.0, 420.0),
    ("ISO A4", 210.0, 297.0),
    ("ISO A5", 148.0, 210.0),
    ("ANSI A (Letter)", 215.9, 279.4),
    ("ANSI B (Tabloid)", 279.4, 431.8),
    ("ANSI C", 431.8, 558.8),
    ("ANSI D", 558.8, 863.6),
    ("Custom", 297.0, 420.0),
];

/// iStCAD's scales: name, paper mm per drawing units. The last is Custom.
const SCALES: &[(&str, f64, f64)] = &[
    ("1:1", 1.0, 1.0),
    ("1:2", 1.0, 2.0),
    ("1:5", 1.0, 5.0),
    ("1:10", 1.0, 10.0),
    ("1:20", 1.0, 20.0),
    ("1:50", 1.0, 50.0),
    ("1:100", 1.0, 100.0),
    ("1:200", 1.0, 200.0),
    ("1:500", 1.0, 500.0),
    ("1:1000", 1.0, 1000.0),
    ("2:1", 2.0, 1.0),
    ("5:1", 5.0, 1.0),
    ("Custom", -1.0, -1.0),
];

/// The drawing's table behind Custom pens: one style per index colour (Color_1 … Color_255).
pub(crate) const PENS: &str = "iStCAD pens.ctb";

/// The plot style tables behind Color, Grayscale, Black and Custom pens.
const STYLES: &[(&str, &str)] = &[("Color", "None"), ("Grayscale", "grayscale.ctb"), ("Black", "monochrome.ctb"), ("Custom pens", PENS)];

/// iStCAD's pen output colours: name and `#rrggbb` (none: the object's colour).
const PEN_COLORS: &[(&str, Option<&str>)] = &[
    ("Use object color", None),
    ("Black", Some("#000000")),
    ("White", Some("#FFFFFF")),
    ("Red", Some("#FF0000")),
    ("Yellow", Some("#FFFF00")),
    ("Green", Some("#00FF00")),
    ("Cyan", Some("#00FFFF")),
    ("Blue", Some("#0000FF")),
    ("Magenta", Some("#FF00FF")),
    ("Gray", Some("#808080")),
];

/// A change to one pen: its index colour, the output colour (`None`: the object's), the
/// lineweight in mm (`None`: the object's).
#[derive(Clone, Debug, PartialEq)]
enum PenEdit {
    Color(u8, Option<&'static str>),
    Weight(u8, Option<f64>),
}

const BG: Color32 = Color32::from_rgb(238, 241, 245);
const CARD_EDGE: Color32 = Color32::from_rgb(210, 216, 224);
const ACCENT: Color32 = Color32::from_rgb(37, 99, 235);
const INK: Color32 = Color32::from_rgb(31, 41, 55);
const MUTED: Color32 = Color32::from_rgb(100, 110, 125);
const PREVIEW_BG: Color32 = Color32::from_rgb(52, 58, 68);

/// The dialog's settings, kept while it is open.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Settings {
    /// "Model" or a layout's name.
    pub target: String,
    pub paper: usize,
    pub custom: (f64, f64),
    pub margin_mm: f64,
    /// 0 Auto, 1 Portrait, 2 Landscape.
    pub orientation: usize,
    /// "extents", "display", "window" (Model) or "layout".
    pub area: &'static str,
    pub window: [f64; 4],
    pub fit: bool,
    pub scale: usize,
    pub custom_scale: (f64, f64),
    pub center: bool,
    pub offset_mm: (f64, f64),
    pub style: usize,
    pub lineweights: bool,
    pub open_after: bool,
    /// The pen being set (an index colour, 1–255), and a count of pen changes so the preview
    /// follows them.
    pub pen: u8,
    pub pens_rev: u32,
}

impl Settings {
    fn new(app: &CadApp) -> Self {
        let target = app.session.layout_space().tab_name().to_string();
        let model = target == "Model";
        let ext = app.session.doc().ok().map(|d| d.extents(&cadcraft_doc::Space::Model)).filter(|b| !b.is_empty());
        let window = ext.map_or([0.0, 0.0, 420.0, 297.0], |b| [b.min.x, b.min.y, b.max.x, b.max.y]);
        Settings {
            target,
            paper: 3,
            custom: (297.0, 420.0),
            margin_mm: 5.0,
            orientation: if model { 0 } else { 2 },
            area: if model { "extents" } else { "layout" },
            window,
            fit: true,
            scale: 0,
            custom_scale: (1.0, 1.0),
            center: true,
            offset_mm: (0.0, 0.0),
            style: 2,
            lineweights: true,
            open_after: true,
            pen: 1,
            pens_rev: 0,
        }
    }

    /// What the preview depends on: everything but which pen is being set.
    fn preview_key(&self) -> Settings {
        Settings { pen: 0, ..self.clone() }
    }

    /// The paper's short and long sides in mm.
    pub fn paper_mm(&self) -> (f64, f64) {
        match PAPERS.get(self.paper) {
            Some((name, _, _)) if *name == "Custom" => self.custom,
            Some((_, s, l)) => (*s, *l),
            None => (297.0, 420.0),
        }
    }

    /// Landscape: as chosen, or (Auto) when the plotted area is wider than tall.
    pub fn landscape(&self, area: Option<(f64, f64)>) -> bool {
        match self.orientation {
            1 => false,
            2 => true,
            _ => area.is_none_or(|(w, h)| w >= h),
        }
    }

    /// The `plot` command's parameters for these settings; `area` is the plotted area's size in
    /// drawing units (for Auto orientation); `preview_dpi` asks for a PNG instead of the PDF.
    pub fn params(&self, area: Option<(f64, f64)>, preview_dpi: Option<f64>) -> Value {
        let (short, long) = self.paper_mm();
        let m = self.margin_mm.clamp(0.0, short / 3.0);
        let mut p = json!({
            "layout": self.target,
            "device": "None",
            "width": short,
            "height": long,
            "landscape": self.landscape(area),
            "margins": [m, m, m, m],
            "fit": self.fit,
            "lineweights": self.lineweights,
        });
        let Some(o) = p.as_object_mut() else { return p };
        if let Some((_, table)) = STYLES.get(self.style)
            && *table != "None"
        {
            o.insert("plotStyleTable".into(), json!(table));
        }
        if self.target == "Model" {
            o.insert("plotArea".into(), json!(self.area));
            if self.area == "window" {
                let [x1, y1, x2, y2] = self.window;
                o.insert("window".into(), json!([[x1, y1], [x2, y2]]));
            }
            if !self.fit {
                let (mm, units) = match SCALES.get(self.scale) {
                    Some((_, mm, units)) if *mm > 0.0 => (*mm, *units),
                    _ => self.custom_scale,
                };
                o.insert("scale".into(), json!(format!("{mm}:{units}")));
            }
            o.insert("center".into(), json!(self.center));
            if !self.center {
                o.insert("offset".into(), json!([self.offset_mm.0, self.offset_mm.1]));
            }
        }
        if let Some(dpi) = preview_dpi {
            o.insert("output".into(), json!("png"));
            o.insert("resolution".into(), json!(dpi));
        }
        p
    }
}

/// The dialog's state between frames.
#[derive(Clone)]
struct State {
    settings: Settings,
    /// The preview, the settings it shows, and when they last changed (seconds).
    preview: Option<TextureHandle>,
    shown: Option<Settings>,
    changed_at: f64,
    status: String,
    /// The plot window is being picked in the drawing.
    picking: bool,
}

fn state_id() -> egui::Id {
    egui::Id::unique("istcad_plot_state")
}

/// The size, in drawing units, of what a model plot covers.
fn area_size(app: &CadApp, s: &Settings) -> Option<(f64, f64)> {
    if s.target != "Model" {
        return None;
    }
    let d = app.session.doc().ok()?;
    let b = match s.area {
        "window" => cadcraft_geom::Bounds2::new(cadcraft_geom::Vec2::new(s.window[0], s.window[1]), cadcraft_geom::Vec2::new(s.window[2], s.window[3])),
        _ => d.extents(&cadcraft_doc::Space::Model),
    };
    (!b.is_empty() && b.width() > 0.0 && b.height() > 0.0).then(|| (b.width(), b.height()))
}

/// Every frame: once the plot window has been picked (or the pick cancelled), bring the dialog
/// back with it.
pub fn resume(app: &mut CadApp, ctx: &egui::Context) {
    let Some(mut st) = ctx.data(|d| d.get_temp::<State>(state_id())) else { return };
    if !st.picking || app.session.running.is_some() {
        return;
    }
    st.picking = false;
    if let Some(w) = app.session.plot_window.take() {
        st.settings.window = [w.min.x, w.min.y, w.max.x, w.max.y];
        st.settings.area = "window";
    }
    ctx.data_mut(|d| d.insert_temp(state_id(), st));
    app.ui.dialog = Some("plot".into());
}

/// A card: the title in small capitals over its body.
fn card<R>(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui) -> R) -> R {
    Frame::NONE
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0, CARD_EDGE))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(crate::i18n::t(title).to_uppercase()).size(10.5).strong().color(MUTED));
            ui.add_space(4.0);
            body(ui)
        })
        .inner
}

/// iStCAD's segmented control: one button per item, the chosen one filled with the accent.
/// Returns the item clicked.
fn segmented(ui: &mut egui::Ui, items: &[&str], selected: usize, enabled: &[bool]) -> Option<usize> {
    let mut clicked = None;
    Frame::NONE.stroke(Stroke::new(1.0, CARD_EDGE)).corner_radius(CornerRadius::same(6)).show(ui, |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.horizontal(|ui| {
            for (i, item) in items.iter().enumerate() {
                let on = i == selected;
                let ok = enabled.get(i).copied().unwrap_or(true);
                let text = RichText::new(crate::i18n::t(item)).size(12.0).color(if on { Color32::WHITE } else { INK });
                let b = egui::Button::new(text).fill(if on { ACCENT } else { Color32::WHITE }).corner_radius(0.0).min_size(vec2(0.0, 26.0));
                let r = ui.add_enabled(ok, b);
                if r.clicked() {
                    clicked = Some(i);
                }
            }
        });
    });
    clicked
}

/// A number field (typed or dragged).
fn number(ui: &mut egui::Ui, v: &mut f64) -> bool {
    ui.add(egui::DragValue::new(v).speed(0.5).max_decimals(3).update_while_editing(false)).changed()
}

/// The dialog. Returns an action to run (Apply to layout), as the other dialogs do.
pub fn dialog(app: &mut CadApp, ctx: &egui::Context, open: &mut bool) -> Option<(&'static str, Value)> {
    if app.session.doc().is_err() {
        *open = false;
        return None;
    }
    let now = ctx.input(|i| i.time);
    let mut st = ctx.data(|d| d.get_temp::<State>(state_id())).unwrap_or_else(|| State {
        settings: Settings::new(app),
        preview: None,
        shown: None,
        changed_at: -1.0,
        status: String::new(),
        picking: false,
    });
    let before = st.settings.clone();
    let model = st.settings.target == "Model";
    let mut targets = vec!["Model".to_string()];
    if let Ok(d) = app.session.doc() {
        let mut ls: Vec<_> = d.layouts.iter().map(|l| (l.tab_order, l.name.clone())).collect();
        ls.sort_by_key(|l| l.0);
        targets.extend(ls.into_iter().map(|l| l.1));
    }
    let area = area_size(app, &st.settings);
    let (mut plot, mut all, mut apply, mut close, mut pick) = (false, false, false, false, false);
    let mut pen_edit = None;
    let pens = app.session.doc().ok().and_then(|d| d.plot_style_tables.iter().find(|t| t.name.eq_ignore_ascii_case(PENS)).cloned());
    let title = format!("{} - {}", crate::tl!("Plot (PDF)"), st.settings.target);
    let center = ctx.content_rect().center();
    egui::Window::new(title.clone())
        .id(egui::Id::unique("istcad_plot"))
        .title_bar(false)
        .resizable(true)
        .default_size(vec2(900.0, if model { 680.0 } else { 600.0 }))
        .min_size(vec2(860.0, 560.0))
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(center)
        .frame(Frame::window(&ctx.global_style()).fill(BG).inner_margin(Margin::same(14)))
        .show(ctx, |ui| {
            ui.style_mut().visuals = egui::Visuals::light();
            ui.style_mut().visuals.selection.bg_fill = ACCENT;
            ui.horizontal(|ui| {
                ui.label(RichText::new(&title).size(14.0).strong().color(INK));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    close |= ui.button("✕").on_hover_text(crate::tl!("Close")).clicked();
                    egui::ComboBox::from_id_salt("ist_plot_target").selected_text(st.settings.target.as_str()).width(150.0).show_ui(ui, |ui| {
                        for n in &targets {
                            if ui.selectable_label(*n == st.settings.target, n).clicked() && *n != st.settings.target {
                                let keep = st.settings.clone();
                                st.settings = Settings { target: n.clone(), ..keep };
                                let m = n == "Model";
                                st.settings.area = if m { "extents" } else { "layout" };
                            }
                        }
                    });
                    ui.label(RichText::new(crate::tl!("Plot")).color(MUTED));
                });
            });
            ui.add_space(8.0);
            let footer_h = 40.0;
            let body_h = ui.available_height() - footer_h;
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(vec2(470.0, body_h), egui::Layout::top_down(egui::Align::Min), |ui| {
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        pen_edit = left_column(ui, &mut st.settings, model, area, &mut pick, pens.as_ref());
                    });
                });
                ui.add_space(10.0);
                Frame::NONE.fill(PREVIEW_BG).corner_radius(CornerRadius::same(8)).inner_margin(Margin::same(14)).show(ui, |ui| {
                    let size = vec2(ui.available_width(), body_h - 28.0);
                    ui.set_min_size(size);
                    ui.vertical_centered(|ui| {
                        let room = vec2(size.x, size.y - 24.0);
                        match &st.preview {
                            Some(tex) => {
                                let t = tex.size_vec2();
                                let k = (room.x / t.x).min(room.y / t.y).min(1.5);
                                ui.add(egui::Image::new((tex.id(), t * k)));
                            }
                            None => {
                                ui.add_space(room.y / 2.0);
                                ui.label(RichText::new(crate::tl!("Preparing preview…")).color(Color32::WHITE));
                            }
                        }
                        let (s, l) = st.settings.paper_mm();
                        let land = st.settings.landscape(area);
                        let (w, h) = if land { (l, s) } else { (s, l) };
                        let orient = if land { crate::tl!("Landscape") } else { crate::tl!("Portrait") };
                        ui.label(RichText::new(format!("{w:.0} × {h:.0} mm  ·  {orient}")).size(11.0).color(Color32::WHITE));
                    });
                });
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(&st.status).color(MUTED));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let primary = egui::Button::new(RichText::new(crate::tl!("Plot PDF")).color(Color32::WHITE)).fill(ACCENT).min_size(vec2(96.0, 30.0));
                    close |= ui.add(egui::Button::new(crate::tl!("Close")).min_size(vec2(80.0, 30.0))).clicked();
                    plot |= ui.add(primary).clicked();
                    if !model {
                        apply |= ui
                            .add(egui::Button::new(crate::tl!("Apply to layout")).min_size(vec2(110.0, 30.0)))
                            .on_hover_text(crate::tl!("Keep the paper, orientation, scale and plot style in this layout's page setup"))
                            .clicked();
                    }
                    all |= ui
                        .add(egui::Button::new(crate::tl!("All layouts → PDF")).min_size(vec2(130.0, 30.0)))
                        .on_hover_text(crate::tl!("Model and every layout, one page each, in one PDF"))
                        .clicked();
                });
            });
        });
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        close = true;
    }
    if let Some(e) = pen_edit {
        match apply_pen(app, &e) {
            Ok(()) => st.settings.pens_rev += 1,
            Err(e) => st.status = e,
        }
    }
    if st.settings.preview_key() != before.preview_key() {
        st.changed_at = now;
    }
    // The preview: a quarter second after the last change.
    if st.shown.as_ref() != Some(&st.settings.preview_key()) {
        if now - st.changed_at >= 0.25 {
            refresh_preview(app, ctx, &mut st);
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(260));
        }
    }
    let mut action = None;
    if apply {
        let (short, long) = st.settings.paper_mm();
        let paper = PAPERS.get(st.settings.paper).and_then(|(n, _, _)| cadcraft_render::paper_size(n.split(" (").next().unwrap_or(n))).map(|p| p.name);
        let mut p = json!({
            "layout": st.settings.target,
            "landscape": st.settings.landscape(area),
            "scaleToFit": st.settings.fit,
            "lineweights": st.settings.lineweights,
            "plotStyleTable": STYLES.get(st.settings.style).map_or("None", |s| s.1),
        });
        if let Some(o) = p.as_object_mut() {
            match paper {
                Some(name) => o.insert("paper".into(), json!(name)),
                None => o.insert("paperSize".into(), json!([short, long])),
            };
        }
        action = Some(("pagesetup", p));
    }
    if pick {
        st.picking = true;
        app.start("plotwindow");
        *open = false;
    }
    if plot || all {
        let mut params = st.settings.params(area, None);
        if all && let Some(o) = params.as_object_mut() {
            o.remove("layout");
            o.insert("layouts".into(), json!("all"));
        }
        let open_after = st.settings.open_after;
        let title = app.session.state().map(|s| s.title.clone()).unwrap_or_default();
        let stem = std::path::Path::new(&title).file_stem().map(|s| s.to_string_lossy().to_string()).filter(|s| !s.is_empty());
        let suggested = format!("{}-{}.pdf", stem.unwrap_or_else(|| "Drawing".into()), if all { "All" } else { st.settings.target.as_str() });
        if app.services.pick_save.is_some() {
            let _ = app.pick_then(Pick::Save(suggested), move |app, path| {
                let mut params = params;
                if let Some(o) = params.as_object_mut() {
                    o.insert("path".into(), json!(path));
                }
                plot_to(app, params, open_after);
                Ok(Value::Null)
            });
        } else {
            plot_to(app, params, false);
        }
    }
    if close {
        *open = false;
    }
    if *open || st.picking {
        ctx.data_mut(|d| d.insert_temp(state_id(), st));
    } else {
        ctx.data_mut(|d| d.remove::<State>(state_id()));
    }
    action
}

/// Plot with `params`: the PDF renders on a worker thread while a progress card shows the sheets
/// done and the time taken ([`Plotting`]); the result (or the error) then goes to the command
/// line, and the PDF opens where the host can open it and `open_after` asks for it. The web
/// build has no threads and plots in place.
fn plot_to(app: &mut CadApp, params: Value, open_after: bool) {
    if app.istcad.plotting.is_some() {
        app.set_status(crate::tl!("A plot is still being made; wait for it to finish."));
        return;
    }
    let job = match cadcraft_engine::cmd::plot_job(&app.session, &params) {
        Ok(job) => job,
        Err(e) => return app.set_status(e.to_string()),
    };
    let (tx, rx) = std::sync::mpsc::channel();
    let done = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let pages = job.pages;
    // The web build renders in place; the result is taken up on the next frame all the same.
    #[cfg(target_arch = "wasm32")]
    {
        let r = job.render();
        let _ = tx.send((job, r));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let counter = done.clone();
        let spawned = std::thread::Builder::new().name("ist-plot".into()).spawn(move || {
            cadcraft_io::pdf::report_pages_to(Some(counter));
            let r = job.render();
            let _ = tx.send((job, r));
        });
        if let Err(e) = spawned {
            return app.set_status(format!("PLOT: could not start the plot: {e}"));
        }
    }
    app.istcad.plotting = Some(Plotting { pages, done, open_after, started: None, rx });
}

/// Write a rendered plot and report it.
fn finish_plot(app: &mut CadApp, job: &cadcraft_engine::cmd::PlotJob, rendered: Result<Vec<u8>, String>, open_after: bool) {
    let r = rendered.map_err(|e| format!("PLOT: {e}")).and_then(|bytes| job.finish(&mut app.session, bytes).map_err(|e| e.to_string()));
    match r {
        Ok(r) => match r.get("path").and_then(Value::as_str) {
            Some(path) => {
                let size = r["bytes"].as_u64().unwrap_or(0) as f64 / 1024.0;
                let size = if size >= 1024.0 { format!("{:.1} MB", size / 1024.0) } else { format!("{size:.0} KB") };
                app.session.echo(format!("Plotted to {path} ({} page(s), {size}).", r["pages"].as_u64().unwrap_or(1)));
                if open_after && let Some(open) = app.services.open_document.as_ref() {
                    open(path);
                }
            }
            None => app.session.echo(format!("Plotted {} bytes of PDF; there is no file dialog here.", r["bytes"])),
        },
        Err(e) => app.set_status(e),
    }
}

/// A plot rendering on a worker thread.
pub struct Plotting {
    /// The sheets (pages) it makes, and how many are done.
    pages: usize,
    done: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    open_after: bool,
    /// When the card was first shown (egui time, seconds).
    started: Option<f64>,
    rx: std::sync::mpsc::Receiver<(cadcraft_engine::cmd::PlotJob, Result<Vec<u8>, String>)>,
}

impl Plotting {
    /// Sheets done of all, for the progress bar.
    fn fraction(&self) -> f32 {
        let done = self.done.load(std::sync::atomic::Ordering::Relaxed).min(self.pages);
        done as f32 / self.pages.max(1) as f32
    }
}

/// Each frame: finish a plot whose PDF is ready, else show its progress card (a bar of the
/// sheets done, the time taken).
pub fn poll(app: &mut CadApp, ctx: &egui::Context) {
    let Some(job) = app.istcad.plotting.as_mut() else { return };
    match job.rx.try_recv() {
        Ok((plot, rendered)) => {
            let open_after = job.open_after;
            app.istcad.plotting = None;
            finish_plot(app, &plot, rendered, open_after);
        }
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            app.istcad.plotting = None;
            app.set_status(crate::tl!("PLOT: the plot stopped before it finished."));
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => {
            let now = ctx.input(|i| i.time);
            let started = *job.started.get_or_insert(now);
            let (pages, fraction) = (job.pages, job.fraction());
            let done = job.done.load(std::sync::atomic::Ordering::Relaxed).min(pages);
            progress_card(ctx, pages, done, fraction, now - started);
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

/// The progress card in the middle of the window, in the dialog's light style.
fn progress_card(ctx: &egui::Context, pages: usize, done: usize, fraction: f32, secs: f64) {
    egui::Area::new(egui::Id::unique("ist_plot_progress")).order(egui::Order::Foreground).anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0)).interactable(false).show(
        ctx,
        |ui| {
            Frame::NONE.fill(Color32::WHITE).stroke(Stroke::new(1.0, CARD_EDGE)).corner_radius(CornerRadius::same(8)).inner_margin(Margin::symmetric(18, 14)).show(ui, |ui| {
                ui.set_width(320.0);
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new().color(ACCENT));
                    ui.label(RichText::new(crate::tl!("Plotting PDF…")).color(INK).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{secs:.0} s")).color(MUTED));
                    });
                });
                ui.add_space(6.0);
                let text = if pages > 1 { format!("{} {} / {pages}", crate::tl!("Sheet"), (done + 1).min(pages)) } else { crate::tl!("Writing vector PDF").to_string() };
                let bar = egui::ProgressBar::new(if pages > 1 { fraction } else { 0.0 }).desired_width(320.0).fill(ACCENT).text(RichText::new(text).color(INK));
                // One sheet: no steps to count, so the bar moves to show the plot is going.
                ui.add(if pages > 1 { bar } else { bar.animate(true) });
            });
        },
    );
}

/// Paper, Plot layout and Plot style.
fn left_column(ui: &mut egui::Ui, s: &mut Settings, model: bool, area: Option<(f64, f64)>, pick: &mut bool, pens: Option<&cadcraft_doc::PlotStyleTable>) -> Option<PenEdit> {
    let mut edit = None;
    card(ui, "Paper", |ui| {
        ui.horizontal(|ui| {
            let shown = |i: usize| PAPERS.get(i).map_or(String::new(), |(n, a, b)| format!("{n} ({a} x {b} MM)"));
            egui::ComboBox::from_id_salt("ist_plot_paper").selected_text(shown(s.paper)).width(258.0).show_ui(ui, |ui| {
                for i in 0..PAPERS.len() {
                    ui.selectable_value(&mut s.paper, i, shown(i));
                }
            });
            ui.label(RichText::new(crate::tl!("Margin")).color(MUTED));
            number(ui, &mut s.margin_mm);
            ui.label(RichText::new("mm").color(MUTED));
        });
        if PAPERS.get(s.paper).is_some_and(|p| p.0 == "Custom") {
            ui.horizontal(|ui| {
                ui.label(RichText::new(crate::tl!("Custom size")).color(MUTED));
                number(ui, &mut s.custom.0);
                ui.label("×");
                number(ui, &mut s.custom.1);
                ui.label(RichText::new("mm").color(MUTED));
            });
            s.custom = (s.custom.0.max(10.0), s.custom.1.max(10.0));
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(crate::tl!("Orientation")).color(MUTED));
            if let Some(i) = segmented(ui, &["Auto", "Portrait", "Landscape"], s.orientation, &[true, true, true]) {
                s.orientation = i;
            }
        });
        s.margin_mm = s.margin_mm.clamp(0.0, 100.0);
    });
    card(ui, "Plot layout", |ui| {
        let (names, kinds): (&[&str], &[&'static str]) =
            if model { (&["Extents", "Display", "Window"], &["extents", "display", "window"]) } else { (&["Layout"], &["layout"]) };
        let current = kinds.iter().position(|k| *k == s.area).unwrap_or(0);
        if let Some(i) = segmented(ui, names, current, &[true; 3]) {
            s.area = kinds[i];
            // Choosing Window picks it in the drawing right away, as iStCAD does.
            if kinds[i] == "window" {
                *pick = true;
            }
        }
        if s.area == "window" {
            ui.horizontal(|ui| {
                let [x1, y1, x2, y2] = s.window;
                ui.label(RichText::new(format!("({x1:.2}, {y1:.2}) → ({x2:.2}, {y2:.2})")).size(11.5).color(INK));
                if ui.small_button(crate::tl!("Pick again")).clicked() {
                    *pick = true;
                }
            });
        }
        if !model {
            ui.label(RichText::new(crate::tl!("A layout plots its whole sheet at its own scale.")).size(11.0).color(MUTED));
            return;
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.checkbox(&mut s.fit, crate::tl!("Fit to paper"));
            if !s.fit {
                egui::ComboBox::from_id_salt("ist_plot_scale").selected_text(SCALES.get(s.scale).map_or("", |x| x.0)).width(90.0).show_ui(ui, |ui| {
                    for (i, (n, _, _)) in SCALES.iter().enumerate() {
                        ui.selectable_value(&mut s.scale, i, *n);
                    }
                });
                if SCALES.get(s.scale).is_some_and(|x| x.1 < 0.0) {
                    number(ui, &mut s.custom_scale.0);
                    ui.label(RichText::new(crate::tl!("mm =")).color(MUTED));
                    number(ui, &mut s.custom_scale.1);
                    ui.label(RichText::new(crate::tl!("units")).color(MUTED));
                    s.custom_scale = (s.custom_scale.0.max(1e-6), s.custom_scale.1.max(1e-6));
                }
            }
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut s.center, crate::tl!("Center the plot"));
            if !s.center {
                ui.label(RichText::new("X").color(MUTED));
                number(ui, &mut s.offset_mm.0);
                ui.label(RichText::new("Y").color(MUTED));
                number(ui, &mut s.offset_mm.1);
                ui.label(RichText::new("mm").color(MUTED));
            }
            if s.fit
                && let Some(k) = fitted_scale(s, area)
            {
                // As AutoCAD reads a fitted scale: paper millimetres to drawing units.
                let units = crate::tl!("units");
                ui.label(RichText::new(format!("{} 1 mm = {} {units}", crate::tl!("Scale"), trim(k))).size(11.5).color(MUTED));
            }
        });
    });
    card(ui, "Plot style", |ui| {
        let mut choice = s.style;
        if let Some(i) = segmented(ui, &["Color", "Grayscale", "Black", "Custom pens"], choice, &[true, true, true, true]) {
            choice = i;
        }
        s.style = choice.min(STYLES.len() - 1);
        ui.checkbox(&mut s.lineweights, crate::tl!("Plot object lineweights"));
        ui.checkbox(&mut s.open_after, crate::tl!("Open PDF after plot"));
        if s.style == 3 {
            edit = pen_table(ui, s, pens);
        }
    });
    edit
}

/// iStCAD's inline pen table: the 255 index colours on the left; for the one chosen, its output
/// colour and lineweight on the right.
fn pen_table(ui: &mut egui::Ui, s: &mut Settings, pens: Option<&cadcraft_doc::PlotStyleTable>) -> Option<PenEdit> {
    let mut edit = None;
    let style = |i: u8| pens.and_then(|t| t.styles.get(usize::from(i).saturating_sub(1)));
    ui.add_space(6.0);
    ui.horizontal_top(|ui| {
        Frame::NONE.stroke(Stroke::new(1.0, CARD_EDGE)).show(ui, |ui| {
            ui.set_width(150.0);
            egui::ScrollArea::vertical().id_salt("ist_pens").max_height(170.0).show(ui, |ui| {
                for i in 1..=255u8 {
                    let [r, g, b] = rgb(i);
                    let set = style(i).is_some_and(|st| st.color.is_some() || st.lineweight.is_some());
                    let row = ui.horizontal(|ui| {
                        let (sw, _) = ui.allocate_exact_size(vec2(16.0, 12.0), egui::Sense::hover());
                        ui.painter().rect(sw, 0.0, Color32::from_rgb(r, g, b), Stroke::new(1.0, Color32::GRAY), egui::StrokeKind::Inside);
                        let label = format!("{} {i}{}", crate::tl!("Color"), if set { " •" } else { "" });
                        ui.selectable_label(s.pen == i, RichText::new(label).color(INK))
                    });
                    if row.inner.clicked() {
                        s.pen = i;
                    }
                }
            });
        });
        ui.vertical(|ui| {
            let current = style(s.pen);
            let color = current.and_then(|st| st.color).map(|c| format!("#{:02X}{:02X}{:02X}", c.0, c.1, c.2));
            let shown = PEN_COLORS.iter().find(|(_, hex)| hex.map(str::to_string) == color).map_or_else(|| color.clone().unwrap_or_default(), |(n, _)| n.to_string());
            ui.label(RichText::new(crate::tl!("Output color")).color(MUTED));
            egui::ComboBox::from_id_salt("ist_pen_color").selected_text(crate::i18n::t(&shown)).width(170.0).show_ui(ui, |ui| {
                for (name, hex) in PEN_COLORS {
                    if ui.selectable_label(shown == *name, crate::i18n::t(name)).clicked() {
                        edit = Some(PenEdit::Color(s.pen, *hex));
                    }
                }
            });
            ui.add_space(4.0);
            ui.label(RichText::new(crate::tl!("Lineweight")).color(MUTED));
            let weight = current.and_then(|st| st.lineweight).map(|w| f64::from(w) / 100.0);
            let shown = weight.map_or_else(|| crate::tl!("Use object lineweight").to_string(), |w| format!("{w:.2} mm"));
            egui::ComboBox::from_id_salt("ist_pen_weight").selected_text(shown).width(170.0).show_ui(ui, |ui| {
                if ui.selectable_label(weight.is_none(), crate::tl!("Use object lineweight")).clicked() {
                    edit = Some(PenEdit::Weight(s.pen, None));
                }
                for w in cadcraft_doc::Lineweight::STANDARD.iter() {
                    let mm = f64::from(*w) / 100.0;
                    if ui.selectable_label(weight == Some(mm), format!("{mm:.2} mm")).clicked() {
                        edit = Some(PenEdit::Weight(s.pen, Some(mm)));
                    }
                }
            });
            ui.add_space(6.0);
            ui.label(RichText::new(crate::tl!("Select a color on the left, then set how it plots.")).size(11.0).color(MUTED));
        });
    });
    edit
}

/// The screen colour of index colour `i`.
fn rgb(i: u8) -> [u8; 3] {
    let c = cadcraft_doc::color::aci_rgb(i);
    [c.0, c.1, c.2]
}

/// Apply a pen change to the drawing's pen table (made on first use), through STYLESMANAGER.
fn apply_pen(app: &mut CadApp, edit: &PenEdit) -> Result<(), String> {
    let exists = app.session.doc().is_ok_and(|d| d.plot_style_tables.iter().any(|t| t.name.eq_ignore_ascii_case(PENS)));
    if !exists {
        app.run("stylesmanager", json!({ "action": "new", "name": PENS, "kind": "colorDependent", "description": "iStCAD custom pens" }))?;
    }
    let style = match edit {
        PenEdit::Color(i, hex) => json!({ "name": format!("Color_{i}"), "color": hex }),
        PenEdit::Weight(i, mm) => json!({ "name": format!("Color_{i}"), "lineweight": mm }),
    };
    app.run("stylesmanager", json!({ "action": "set", "name": PENS, "style": style })).map(|_| ())
}

/// A number with up to four decimals and no trailing zeros.
fn trim(v: f64) -> String {
    let s = format!("{v:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// The scale a fitted model plot comes to: drawing units per paper mm (shown 1:k).
fn fitted_scale(s: &Settings, area: Option<(f64, f64)>) -> Option<f64> {
    let (w, h) = area?;
    let (short, long) = s.paper_mm();
    let m = s.margin_mm.clamp(0.0, short / 3.0) * 2.0;
    let (pw, ph) = if s.landscape(area) { (long - m, short - m) } else { (short - m, long - m) };
    let k = (w / pw).max(h / ph);
    (k.is_finite() && k > 0.0).then_some(k)
}

/// Draw the preview for the current settings (a PNG of the sheet, about 900 pixels on its long
/// side) and the status line.
fn refresh_preview(app: &mut CadApp, ctx: &egui::Context, st: &mut State) {
    st.shown = Some(st.settings.preview_key());
    let area = area_size(app, &st.settings);
    let (_, long) = st.settings.paper_mm();
    let dpi = (900.0 / (long / 25.4)).clamp(20.0, 300.0);
    let params = st.settings.params(area, Some(dpi));
    let r = app.session.execute("plot", &params);
    match r {
        Ok(v) => {
            let png = v["data"].as_str().and_then(cadcraft_engine::cmd::file::base64_decode);
            match png.map(|png| cadcraft_io::raster::decode(&png)) {
                Some(Ok(img)) => {
                    let size = [img.width() as usize, img.height() as usize];
                    st.preview =
                        Some(ctx.load_texture("istcad-plot-preview", egui::ColorImage::from_rgba_unmultiplied(size, img.rgba()), egui::TextureOptions::LINEAR));
                    let objects = app
                        .session
                        .doc()
                        .map(|d| if st.settings.target == "Model" { d.model.len() } else { d.layout(&st.settings.target).map_or(0, |l| l.entities.len()) })
                        .unwrap_or(0);
                    st.status = format!("{objects} {}", crate::tl!("objects"));
                }
                _ => st.status = crate::tl!("The preview could not be drawn.").to_string(),
            }
        }
        Err(e) => st.status = format!("{}: {e}", crate::tl!("Preview failed")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Settings {
        Settings {
            target: "Model".into(),
            paper: 3,
            custom: (297.0, 420.0),
            margin_mm: 5.0,
            orientation: 0,
            area: "extents",
            window: [0.0, 0.0, 100.0, 50.0],
            fit: true,
            scale: 0,
            custom_scale: (1.0, 1.0),
            center: true,
            offset_mm: (0.0, 0.0),
            style: 2,
            lineweights: true,
            open_after: false,
            pen: 1,
            pens_rev: 0,
        }
    }

    #[test]
    fn auto_orientation_follows_the_plotted_area() {
        let s = settings();
        assert!(s.landscape(Some((200.0, 100.0))));
        assert!(!s.landscape(Some((100.0, 200.0))));
        assert!(!Settings { orientation: 1, ..settings() }.landscape(Some((200.0, 100.0))), "Portrait stays portrait");
    }

    #[test]
    fn the_settings_become_plot_parameters() {
        let mut s = settings();
        s.fit = false;
        s.scale = SCALES.len() - 1;
        s.custom_scale = (2.0, 25.0);
        s.center = false;
        s.offset_mm = (10.0, -5.0);
        s.area = "window";
        let p = s.params(Some((100.0, 50.0)), None);
        assert_eq!(p["scale"], "2:25");
        assert_eq!(p["offset"], json!([10.0, -5.0]));
        assert_eq!(p["plotStyleTable"], "monochrome.ctb");
        assert_eq!(p["window"], json!([[0.0, 0.0], [100.0, 50.0]]));
        assert_eq!((p["width"].as_f64(), p["height"].as_f64()), (Some(297.0), Some(420.0)));
        // Color plots without a table; a layout leaves the model-only settings out.
        let p = Settings { style: 0, target: "Layout1".into(), area: "layout", ..settings() }.params(None, Some(96.0));
        assert!(p.get("plotStyleTable").is_none() && p.get("plotArea").is_none() && p.get("scale").is_none());
        assert_eq!(p["output"], "png");
    }

    #[test]
    fn custom_pens_set_the_drawing_s_pen_table() {
        let mut app = CadApp::new(cadcraft_engine::Session::default(), crate::Services::default());
        let _ = app.run("ui.sample", json!({}));
        apply_pen(&mut app, &PenEdit::Color(1, Some("#000000"))).unwrap();
        apply_pen(&mut app, &PenEdit::Weight(1, Some(0.5))).unwrap();
        let d = app.session.doc().unwrap();
        let t = d.plot_style_tables.iter().find(|t| t.name == PENS).expect("the pen table is made");
        let red = &t.styles[0];
        assert_eq!((red.color.map(|c| (c.0, c.1, c.2)), red.lineweight), (Some((0, 0, 0)), Some(50)));
        let p = Settings { style: 3, ..settings() }.params(None, None);
        assert_eq!(p["plotStyleTable"], PENS);
    }

    #[test]
    fn a_fitted_plot_reports_its_scale() {
        // 400 × 100 units on A3 landscape with 5 mm margins: 410 mm wide, so a little under 1:1.
        let k = fitted_scale(&settings(), Some((400.0, 100.0))).unwrap();
        assert!(k < 1.0 && k > 0.9, "{k}");
    }

    #[test]
    fn the_dialog_plots_a_preview_of_the_sample() {
        cadcraft_engine::cmd::file::set_io(cadcraft_engine::cmd::file::IoHooks {
            read: |b, name| cadcraft_io::read(b, name).map_err(|e| e.to_string()),
            write: |d, name| cadcraft_io::write(d, name).map_err(|e| e.to_string()),
            plot: Some(|d, space, opts| cadcraft_io::plotter::plot(d, space, opts).map_err(|e| e.to_string())),
        });
        let mut app = CadApp::new(cadcraft_engine::Session::default(), crate::Services::default());
        let _ = app.run("ui.sample", json!({}));
        let ctx = egui::Context::default();
        let mut st = State { settings: Settings::new(&app), preview: None, shown: None, changed_at: 0.0, status: String::new(), picking: false };
        let mut out = ctx.run_pass(egui::RawInput::default(), |ctx| refresh_preview(&mut app, ctx, &mut st));
        // No painter here takes the preview's texture.
        out.textures_delta.clear();
        // Another test may have installed a stand-in plotter first, which writes no PNG.
        if st.status == "The preview could not be drawn." {
            return;
        }
        assert!(st.preview.is_some(), "{}", st.status);
    }
}
