//! iStCAD's Properties palette (Ctrl+1): a 296-point column on the right of the drawing area,
//! hidden until asked for, as iStCAD's `PropertiesPanel` is.
//!
//! Layout (iStCAD 1.0.233 `MainWindow.xaml`): `#313741`, padding 6, its left edge dragged to
//! resize; the "PROPERTIES" title; the selection box ("No selection", the object type or "N
//! objects selected", ⌄ to limit the rows to one type); then the groups General, Geometry,
//! Text, View (closed) and Misc, each under a 24-point `#262B33` header bar with ◀ / ▼. Rows: the
//! label (`#BEC4CD`) in 1 part and the value (`#E1E3E8`, right-aligned) in 1.15 parts.
//!
//! General follows iStCAD's order: Count, Type, Layer, Color, (one object) Handle, Linetype and
//! Lineweight; one object's geometry is under Geometry. The values come from the engine's
//! `properties.describe` and are changed through `properties.set` (geometry through the same
//! keys CADCraft's palette uses), so the palette edits what iStCAD's only showed: click a value
//! to type a new one (Enter applies, Esc cancels) or to pick from its list.

use egui::{Align2, Color32, FontId, Rect, RichText, Sense, Stroke, pos2, vec2};
use serde_json::{Value, json};

use cadcraft_engine::doc::{Drawing, Entity, EntityKind, Handle};
use cadcraft_engine::properties::{self, Kind, Row as DescRow};

use crate::CadApp;
use crate::theme::Tokens;

/// Width of the palette column (iStCAD's `PropertiesColumn` when shown).
pub const WIDTH: f32 = 296.0;
const ROW_H: f32 = 30.0;
/// iStCAD 1.0.233's palette fill (`#313741`) and group header bar (`#262B33`, `#4A525E` rule).
const PANEL_FILL: Color32 = Color32::from_rgb(0x31, 0x37, 0x41);
const GROUP_FILL: Color32 = Color32::from_rgb(0x26, 0x2b, 0x33);
const GROUP_RULE: Color32 = Color32::from_rgb(0x4a, 0x52, 0x5e);
const LABEL_INK: Color32 = Color32::from_rgb(190, 196, 205);
const VALUE_INK: Color32 = Color32::from_rgb(225, 227, 232);
const HEADER_INK: Color32 = Color32::from_rgb(0xe4, 0xe9, 0xea);
const BOX_FILL: Color32 = Color32::from_rgb(0x4a, 0x51, 0x5b);

/// What applying a typed number changes.
#[derive(Clone, Debug, PartialEq)]
enum Target {
    /// A `properties.set` key taking the number.
    Key(&'static str),
    /// The x of a point key; `y` is kept.
    X(&'static str, f64),
    /// The y of a point key; `x` is kept.
    Y(&'static str, f64),
}

/// How a row's value can be changed.
#[derive(Clone, Debug, PartialEq)]
enum Edit {
    ReadOnly,
    Number(Target),
    Text(&'static str),
    /// Pick from `(value sent, label shown)`.
    Choice(&'static str, Vec<(Value, String)>),
    Toggle(&'static str, bool),
    /// The current layer (no selection): LAYER.CURRENT.
    CurrentLayer(Vec<String>),
}

/// One palette row.
#[derive(Clone, Debug, PartialEq)]
struct PropRow {
    label: String,
    value: String,
    edit: Edit,
}

impl PropRow {
    fn new(label: &str, value: impl Into<String>, edit: Edit) -> Self {
        PropRow { label: label.to_string(), value: value.into(), edit }
    }
    fn read(label: &str, value: impl Into<String>) -> Self {
        Self::new(label, value, Edit::ReadOnly)
    }
}

/// A titled group of rows (one expander).
struct Section {
    title: &'static str,
    open_by_default: bool,
    rows: Vec<PropRow>,
}

/// Palette state kept by the shell.
#[derive(Default)]
pub struct State {
    /// The palette is shown (Ctrl+1).
    pub open: bool,
    /// Limit the rows to objects of this type.
    pub filter: Option<String>,
    /// The row being typed into: (row id, text).
    editing: Option<(String, String)>,
    /// Focus the edit field on the frame it opens.
    focus_pending: bool,
}

/// Show or hide the palette (Ctrl+1, PROPERTIES), as iStCAD's `TogglePropertiesPalette`, which
/// also notes it in the command history.
pub fn toggle(app: &mut CadApp, on: Option<bool>) -> Value {
    let open = on.unwrap_or(!app.istcad.props.open);
    app.istcad.props.open = open;
    app.istcad.props.editing = None;
    app.session.echo(format!("Properties palette: {}.", if open { "shown" } else { "hidden" }));
    json!({ "properties": open })
}

/// The palette column; call before the drawing area's central panel.
pub fn panel(app: &mut CadApp, ui: &mut egui::Ui) {
    if !app.istcad.props.open {
        return;
    }
    let t = Tokens::get();
    // iStCAD 1.0.233: `#313741`, padding 6, resizable by dragging its left edge.
    let frame = egui::Frame::NONE.fill(PANEL_FILL).corner_radius(4).inner_margin(egui::Margin::symmetric(6, 0));
    egui::Panel::right("ist_properties").default_size(WIDTH).size_range(220.0..=640.0).resizable(true).frame(frame).show(ui, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(6.0);
            let title = ui.add(egui::Label::new(RichText::new(crate::tl!("PROPERTIES")).size(12.0).strong().color(HEADER_INK)));
            let _ = title.on_hover_text(crate::tl!("Properties palette — Ctrl+1 to show or hide"));
            ui.add_space(6.0);
            let sel = app.session.selection();
            // Everything shown is read from the drawing first, so the rows can then run commands.
            let view = app.session.state().map(|st| st.view()).ok();
            let filter = app.istcad.props.filter.clone();
            let Ok(doc) = app.session.doc() else {
                ui.label(RichText::new(crate::tl!("No drawing open")).color(t.text_dim));
                return;
            };
            let types = type_counts(doc, &sel);
            let filter = filter.filter(|f| types.iter().any(|(n, _)| n == f));
            let handles: Vec<Handle> = match &filter {
                Some(f) => sel.iter().copied().filter(|h| doc.entity(*h).is_some_and(|e| e.kind.type_name() == f)).collect(),
                None => sel.clone(),
            };
            let sections = sections(doc, &handles, view);
            app.istcad.props.filter = filter;
            selection_box(app, ui, &types, sel.len());
            ui.add_space(5.0);
            for section in &sections {
                // A group with nothing to show for this selection is left out, as in iStCAD.
                if !section.rows.is_empty() {
                    section_ui(app, ui, section, &handles);
                }
            }
            ui.add_space(12.0);
        });
    });
}

/// The selected objects' types and how many of each, in the order first selected.
fn type_counts(doc: &Drawing, sel: &[Handle]) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = Vec::new();
    for e in sel.iter().filter_map(|h| doc.entity(*h)) {
        let name = e.kind.type_name();
        match out.iter_mut().find(|(n, _)| n == name) {
            Some((_, c)) => *c += 1,
            None => out.push((name.to_string(), 1)),
        }
    }
    out
}

/// iStCAD's selection summary: "No selection", the one object's type, or "N objects selected".
fn summary(types: &[(String, usize)], count: usize) -> String {
    match (count, types.first()) {
        (0, _) => crate::tl!("No selection").to_string(),
        (1, Some((name, _))) => name.clone(),
        _ => format!("{count} objects selected"),
    }
}

/// The selection box: the summary and ⌄, opening a list of the selected types to limit the rows to.
fn selection_box(app: &mut CadApp, ui: &mut egui::Ui, types: &[(String, usize)], count: usize) {
    let t = Tokens::get();
    let text = match &app.istcad.props.filter {
        Some(f) => format!("{f} ({})", types.iter().find(|(n, _)| n == f).map_or(0, |(_, c)| *c)),
        None => summary(types, count),
    };
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
    ui.painter().rect_filled(rect, 4.0, if resp.hovered() { t.control_hover.gamma_multiply(1.15) } else { BOX_FILL });
    let p = ui.painter().with_clip_rect(rect.shrink(2.0));
    p.text(pos2(rect.left() + 11.0, rect.center().y), Align2::LEFT_CENTER, &text, FontId::proportional(13.0), t.text);
    p.text(pos2(rect.right() - 11.0, rect.center().y - 2.0), Align2::RIGHT_CENTER, "⌄", FontId::proportional(13.0), t.text);
    crate::icons::describe_control(ui, &resp, "Selection filter", None, false);
    let resp = resp.on_hover_text(crate::tl!("Selection filter"));
    let mut pick: Option<Option<String>> = None;
    egui::Popup::menu(&resp).show(|ui| {
        ui.set_min_width(rect.width());
        if types.is_empty() {
            ui.label(RichText::new(crate::tl!("No selection")).color(t.text_dim));
            return;
        }
        if ui.add(egui::Button::selectable(app.istcad.props.filter.is_none(), format!("{} ({count})", crate::tl!("All")))).clicked() {
            pick = Some(None);
            ui.close();
        }
        for (name, c) in types {
            let on = app.istcad.props.filter.as_deref() == Some(name.as_str());
            if ui.add(egui::Button::selectable(on, format!("{name} ({c})"))).clicked() {
                pick = Some(Some(name.clone()));
                ui.close();
            }
        }
    });
    if let Some(f) = pick {
        app.istcad.props.filter = f;
        app.istcad.props.editing = None;
    }
}

// ---------------------------------------------------------------- rows

/// The palette's sections for `handles`: General, the engine's own groups for text, mtext and
/// dimensions under Text, then View and Misc.
fn sections(doc: &Drawing, handles: &[Handle], view: Option<cadcraft_engine::View>) -> Vec<Section> {
    let units = Units::of(doc);
    let desc = (!handles.is_empty()).then(|| properties::describe(doc, handles, None));
    let general_rows = desc.as_ref().and_then(|d| d.groups.iter().find(|g| g.name == "General")).map(|g| g.rows.clone()).unwrap_or_default();
    let row = |key: &str| general_rows.iter().find(|r| r.key == key);
    let mut general = Vec::new();
    let mut geometry = Vec::new();
    let mut misc = Vec::new();
    if handles.is_empty() {
        // iStCAD shows Type and Layer with nothing selected; the layer here is the current one.
        general.push(PropRow::read("Type", "—"));
        let layers: Vec<String> = doc.layers.iter().map(|l| l.name.clone()).collect();
        general.push(PropRow::new("Layer", doc.header.str("CLAYER", "0"), Edit::CurrentLayer(layers)));
    } else {
        let ents: Vec<&Entity> = handles.iter().filter_map(|h| doc.entity(*h).map(|e| &**e)).collect();
        let types = type_counts(doc, handles);
        general.push(PropRow::read("Count", ents.len().to_string()));
        general.push(PropRow::read("Type", if types.len() == 1 { types[0].0.clone() } else { "Multiple".into() }));
        if let Some(r) = row("layer") {
            general.push(from_desc("Layer", r, layer_choices(doc)));
        }
        if let Some(r) = row("color") {
            general.push(from_desc("Color", r, color_choices()));
        }
        if let [e] = ents.as_slice() {
            general.push(PropRow::read("Handle", e.handle.hex()));
            geometry.extend(geometry_rows(e, &units));
        }
        if let Some(r) = row("linetype") {
            general.push(from_desc("Linetype", r, linetype_choices(doc)));
        }
        if let Some(r) = row("lineweight") {
            general.push(from_desc("Lineweight", r, Vec::new()));
        }
        let thickness = shared(ents.iter().map(|e| e.common.thickness));
        misc.push(match thickness {
            Some(v) => PropRow::new("Thickness", units.distance(v), Edit::Number(Target::Key("thickness"))),
            None => PropRow::new("Thickness", "Multiple", Edit::Number(Target::Key("thickness"))),
        });
        for (key, label) in [("ltscale", "Linetype scale"), ("transparency", "Transparency")] {
            if let Some(r) = row(key) {
                misc.push(from_desc(label, r, Vec::new()));
            }
        }
    }
    // iStCAD 1.0.233's groups: General, Geometry, Text, View (closed) and Misc.
    let mut out = vec![Section { title: "General", open_by_default: true, rows: general }];
    out.push(Section { title: "Geometry", open_by_default: true, rows: geometry });
    // Text, MText and Dimension rows, as the engine describes them, under Text.
    let text: Vec<PropRow> =
        desc.iter().flat_map(|d| d.groups.iter()).filter(|g| g.name != "General").flat_map(|g| g.rows.iter().map(|r| from_desc(r.label, r, r_choices(r)))).collect();
    out.push(Section { title: "Text", open_by_default: true, rows: text });
    let view_rows = view
        .map(|v| {
            vec![
                PropRow::read("Center X", units.distance(v.center.x)),
                PropRow::read("Center Y", units.distance(v.center.y)),
                PropRow::read("Height", units.distance(v.height)),
            ]
        })
        .unwrap_or_default();
    out.push(Section { title: "View", open_by_default: false, rows: view_rows });
    out.push(Section { title: "Misc", open_by_default: true, rows: misc });
    out
}

/// The value every item has, or `None` when they differ (or there are none).
fn shared(mut values: impl Iterator<Item = f64>) -> Option<f64> {
    let first = values.next()?;
    values.all(|v| (v - first).abs() < 1e-12).then_some(first)
}

/// A row from the engine's description: its display (*VARIES* reads "Multiple", as in iStCAD)
/// and how it is edited.
fn from_desc(label: &str, r: &DescRow, choices: Vec<(Value, String)>) -> PropRow {
    let value = if r.varies { "Multiple".to_string() } else { r.display.clone() };
    let edit = if !r.editable {
        Edit::ReadOnly
    } else if !choices.is_empty() {
        Edit::Choice(r.key, choices)
    } else {
        match r.kind {
            Kind::Bool => Edit::Toggle(r.key, r.value.as_bool().unwrap_or(false)),
            Kind::Number | Kind::Distance | Kind::Angle | Kind::Direction => Edit::Number(Target::Key(r.key)),
            _ => Edit::Text(r.key),
        }
    };
    PropRow::new(label, value, edit)
}

/// A described row's own options.
fn r_choices(r: &DescRow) -> Vec<(Value, String)> {
    r.options.iter().map(|c| (c.value.clone(), c.label.clone())).collect()
}

fn layer_choices(doc: &Drawing) -> Vec<(Value, String)> {
    doc.layers.iter().map(|l| (json!(l.name), l.name.clone())).collect()
}

fn linetype_choices(doc: &Drawing) -> Vec<(Value, String)> {
    let mut out = vec![(json!("ByLayer"), "ByLayer".to_string()), (json!("ByBlock"), "ByBlock".to_string())];
    out.extend(
        doc.linetypes
            .iter()
            .filter(|l| !l.name.eq_ignore_ascii_case("ByLayer") && !l.name.eq_ignore_ascii_case("ByBlock"))
            .map(|l| (json!(l.name), l.name.clone())),
    );
    out
}

fn color_choices() -> Vec<(Value, String)> {
    let mut out = vec![(json!("ByLayer"), "ByLayer".to_string()), (json!("ByBlock"), "ByBlock".to_string())];
    for (aci, name) in [(1, "Red"), (2, "Yellow"), (3, "Green"), (4, "Cyan"), (5, "Blue"), (6, "Magenta"), (7, "White")] {
        out.push((json!(aci), name.to_string()));
    }
    out
}

/// Lengths and angles as the drawing shows them (LUNITS/LUPREC; angles in degrees, 3 places as
/// iStCAD prints them).
struct Units {
    lunits: i64,
    luprec: i64,
}

impl Units {
    fn of(doc: &Drawing) -> Self {
        Units { lunits: doc.header.i64("LUNITS", 2), luprec: doc.header.i64("LUPREC", 4) }
    }
    fn distance(&self, v: f64) -> String {
        cadcraft_engine::units::format_distance(v, self.lunits, self.luprec)
    }
    fn degrees(&self, radians: f64) -> String {
        format!("{}°", trim(radians.to_degrees()))
    }
}

/// A number with up to three decimals and no trailing zeros ("0.###").
fn trim(v: f64) -> String {
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.to_string() }
}

/// iStCAD's geometry rows for one object, editable where `properties.set` takes the value.
fn geometry_rows(e: &Entity, u: &Units) -> Vec<PropRow> {
    let num = |label: &str, v: f64, target: Target| PropRow::new(label, u.distance(v), Edit::Number(target));
    match &e.kind {
        EntityKind::Line(l) => {
            let (a, b) = (l.a, l.b);
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            vec![
                num("Start X", a.x, Target::X("start", a.y)),
                num("Start Y", a.y, Target::Y("start", a.x)),
                num("End X", b.x, Target::X("end", b.y)),
                num("End Y", b.y, Target::Y("end", b.x)),
                num("Length", dx.hypot(dy), Target::Key("length")),
                PropRow::new("Angle", u.degrees(dy.atan2(dx)), Edit::Number(Target::Key("angle"))),
            ]
        }
        EntityKind::Circle(c) => vec![
            num("Center X", c.center.x, Target::X("center", c.center.y)),
            num("Center Y", c.center.y, Target::Y("center", c.center.x)),
            num("Radius", c.radius, Target::Key("radius")),
            num("Diameter", c.radius.abs() * 2.0, Target::Key("diameter")),
        ],
        EntityKind::Arc(a) => vec![
            PropRow::read("Center X", u.distance(a.center.x)),
            PropRow::read("Center Y", u.distance(a.center.y)),
            num("Radius", a.radius, Target::Key("radius")),
            PropRow::new("Start angle", u.degrees(a.start), Edit::Number(Target::Key("startAngle"))),
            PropRow::new("End angle", u.degrees(a.end), Edit::Number(Target::Key("endAngle"))),
        ],
        EntityKind::LwPolyline(pl) => {
            let pts: Vec<(f64, f64, f64)> = pl.vertices.iter().map(|v| (v.p.x, v.p.y, v.bulge)).collect();
            vec![
                PropRow::read("Closed", if pl.closed { "Yes" } else { "No" }),
                PropRow::read("Vertices", pts.len().to_string()),
                PropRow::read("Length", u.distance(polyline_length(&pts, pl.closed))),
            ]
        }
        _ => Vec::new(),
    }
}

/// Length of a lightweight polyline: chords, or arcs where a vertex has a bulge.
fn polyline_length(pts: &[(f64, f64, f64)], closed: bool) -> f64 {
    let n = pts.len();
    if n < 2 {
        return 0.0;
    }
    let segments = if closed { n } else { n - 1 };
    (0..segments)
        .map(|i| {
            let (x0, y0, bulge) = pts[i];
            let (x1, y1, _) = pts[(i + 1) % n];
            let chord = (x1 - x0).hypot(y1 - y0);
            if bulge.abs() < 1e-12 {
                return chord;
            }
            // Included angle θ = 4·atan(bulge); arc length = chord · θ / (2·sin(θ/2)).
            let theta = 4.0 * bulge.atan();
            let half = (theta / 2.0).sin();
            if half.abs() < 1e-12 { chord } else { (chord * theta / (2.0 * half)).abs() }
        })
        .sum()
}

// ---------------------------------------------------------------- drawing

/// One expander: its header (a chevron and the title, semibold `#E4E9EA`, a `#535A64` rule
/// under it) and, when open, its rows.
fn section_ui(app: &mut CadApp, ui: &mut egui::Ui, s: &Section, handles: &[Handle]) {
    let id = ui.scope_id().with(("ist_prop_section", s.title));
    let open = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(s.open_by_default);
    // iStCAD 1.0.233's group header: a 24-point `#262B33` bar with a `#4A525E` rule, the title
    // semibold, and ◀ (closed) / ▼ (open) at the right.
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::click());
    let p = ui.painter();
    p.rect_filled(rect, 0.0, GROUP_FILL);
    p.hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, GROUP_RULE));
    p.text(pos2(rect.left() + 8.0, rect.center().y), Align2::LEFT_CENTER, crate::i18n::t(s.title), FontId::proportional(12.0), Color32::from_rgb(0xf1, 0xf4, 0xf8));
    let ink = if resp.hovered() { Color32::WHITE } else { Color32::from_rgb(0xc9, 0xcf, 0xd8) };
    let c = pos2(rect.right() - 12.0, rect.center().y);
    let chevron = if open { vec![c + vec2(-4.0, -2.5), c + vec2(4.0, -2.5), c + vec2(0.0, 2.5)] } else { vec![c + vec2(2.5, -4.0), c + vec2(2.5, 4.0), c + vec2(-2.5, 0.0)] };
    p.add(egui::Shape::convex_polygon(chevron, ink, Stroke::NONE));
    crate::icons::describe_control(ui, &resp, s.title, Some(open), false);
    if resp.clicked() {
        ui.data_mut(|d| d.insert_temp(id, !open));
    }
    if !open {
        ui.add_space(2.0);
        return;
    }
    ui.add_space(4.0);
    for (i, row) in s.rows.iter().enumerate() {
        row_ui(app, ui, row, &format!("{}:{i}:{}", s.title, row.label), handles);
    }
    ui.add_space(6.0);
}

/// One row (iStCAD's `AddPropertyRow`), and its editing.
fn row_ui(app: &mut CadApp, ui: &mut egui::Ui, row: &PropRow, row_id: &str, handles: &[Handle]) {
    let t = Tokens::get();
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::click());
    let inner = Rect::from_min_max(pos2(rect.left() + 2.0, rect.top()), pos2(rect.right() - 2.0, rect.bottom()));
    let split = inner.left() + inner.width() / 2.15;
    let value_rect = Rect::from_min_max(pos2(split + 4.0, inner.top() + 3.0), pos2(inner.right() - 2.0, inner.bottom() - 3.0));
    let editable = row.edit != Edit::ReadOnly;
    if editable && resp.hovered() {
        ui.painter().rect_filled(value_rect, 3.0, t.control);
    }
    let p = ui.painter().with_clip_rect(inner);
    let label = clipped(&p, crate::i18n::t(&row.label), split - inner.left() - 12.0, LABEL_INK);
    p.galley(pos2(inner.left() + 8.0, inner.center().y - label.size().y / 2.0), label, LABEL_INK);
    let editing = app.istcad.props.editing.as_ref().is_some_and(|(id, _)| id == row_id);
    if editing {
        edit_field(app, ui, value_rect, row, handles);
    } else {
        let value = clipped(&p, &row.value, value_rect.width() - 13.0, VALUE_INK);
        p.galley(pos2(value_rect.right() - 9.0 - value.size().x, inner.center().y - value.size().y / 2.0), value, VALUE_INK);
    }
    crate::icons::describe_control(ui, &resp, &format!("{}: {}", row.label, row.value), None, false);
    let resp = resp.on_hover_text(format!("{}: {}", row.label, row.value));
    if !editable || editing {
        return;
    }
    match &row.edit {
        Edit::Choice(key, options) => {
            let mut picked = None;
            egui::Popup::menu(&resp).show(|ui| {
                ui.set_min_width(value_rect.width().max(160.0));
                egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                    for (value, label) in options {
                        if ui.add(egui::Button::selectable(*label == row.value, label.as_str())).clicked() {
                            picked = Some(value.clone());
                            ui.close();
                        }
                    }
                });
            });
            if let Some(v) = picked {
                apply(app, handles, json!({ *key: v }));
            }
        }
        Edit::CurrentLayer(layers) => {
            let mut picked = None;
            egui::Popup::menu(&resp).show(|ui| {
                ui.set_min_width(value_rect.width().max(160.0));
                egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                    for name in layers {
                        if ui.add(egui::Button::selectable(*name == row.value, name.as_str())).clicked() {
                            picked = Some(name.clone());
                            ui.close();
                        }
                    }
                });
            });
            if let Some(name) = picked {
                let _ = app.run("layer.current", json!({ "name": name }));
            }
        }
        Edit::Toggle(key, on) => {
            if resp.clicked() {
                apply(app, handles, json!({ *key: !*on }));
            }
        }
        Edit::Number(_) | Edit::Text(_) => {
            if resp.clicked() {
                let start = if row.value == "Multiple" { String::new() } else { row.value.trim_end_matches('°').to_string() };
                app.istcad.props.editing = Some((row_id.to_string(), start));
                app.istcad.props.focus_pending = true;
            }
        }
        Edit::ReadOnly => {}
    }
}

/// `text` on one line no wider than `max`, cut with "…".
fn clipped(p: &egui::Painter, text: &str, max: f32, color: Color32) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_string(), FontId::proportional(13.0), color);
    job.wrap = egui::text::TextWrapping { max_width: max.max(1.0), max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
    p.layout_job(job)
}

/// The field a value is typed into: Enter applies, Esc cancels, leaving the field applies.
fn edit_field(app: &mut CadApp, ui: &mut egui::Ui, rect: Rect, row: &PropRow, handles: &[Handle]) {
    let t = Tokens::get();
    let Some((_, text)) = app.istcad.props.editing.as_mut() else { return };
    ui.painter().rect(rect, 3.0, t.field, Stroke::new(1.0, t.accent), egui::StrokeKind::Inside);
    let edit = egui::TextEdit::singleline(text)
        .frame(egui::Frame::NONE)
        .text_color(VALUE_INK)
        .align(egui::Align2::RIGHT_CENTER)
        .margin(egui::Margin::symmetric(6, 2));
    let resp = ui.put(rect, edit);
    if app.istcad.props.focus_pending {
        resp.request_focus();
        app.istcad.props.focus_pending = false;
    }
    let cancel = ui.input(|i| i.key_pressed(egui::Key::Escape));
    if cancel {
        app.istcad.props.editing = None;
        return;
    }
    if !resp.lost_focus() {
        return;
    }
    let typed = app.istcad.props.editing.take().map(|(_, s)| s).unwrap_or_default();
    let typed = typed.trim().trim_end_matches('°').trim().to_string();
    if typed.is_empty() || typed == row.value.trim_end_matches('°') {
        return;
    }
    let params = match &row.edit {
        Edit::Text(key) => json!({ *key: typed }),
        Edit::Number(target) => {
            let Ok(v) = typed.replace(',', ".").parse::<f64>() else {
                app.set_status(format!("{}: \"{typed}\" is not a number", row.label));
                return;
            };
            match target {
                Target::Key(key) => json!({ *key: v }),
                Target::X(key, y) => json!({ *key: [v, y] }),
                Target::Y(key, x) => json!({ *key: [x, v] }),
            }
        }
        _ => return,
    };
    apply(app, handles, params);
}

/// Run `properties.set` on `handles` with `params`, reporting a refusal in the status slot.
fn apply(app: &mut CadApp, handles: &[Handle], mut params: Value) {
    if let Some(o) = params.as_object_mut() {
        o.insert("handles".into(), json!(handles.iter().map(|h| h.hex()).collect::<Vec<_>>()));
    }
    match app.run("properties.set", params) {
        Ok(v) => {
            if let Some(m) = v.get("message").and_then(Value::as_str) {
                app.set_status(m.to_string());
            }
        }
        Err(e) => app.set_status(e),
    }
    app.canvas.list = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn istcad_polyline_length_follows_bulges() {
        let square = [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (1.0, 1.0, 0.0), (0.0, 1.0, 0.0)];
        assert!((polyline_length(&square, true) - 4.0).abs() < 1e-9);
        assert!((polyline_length(&square, false) - 3.0).abs() < 1e-9);
        // A bulge of 1 makes a half circle over the chord: longer than the chord.
        let half = [(0.0, 0.0, 1.0), (2.0, 0.0, 0.0)];
        assert!((polyline_length(&half, false) - std::f64::consts::PI).abs() < 1e-9);
    }

    #[test]
    fn istcad_numbers_read_like_istcad() {
        assert_eq!(trim(45.0), "45");
        assert_eq!(trim(12.34567), "12.346");
        assert_eq!(trim(-0.0001), "0");
    }

    #[test]
    fn istcad_summary_names_the_selection() {
        assert_eq!(summary(&[], 0), "No selection");
        assert_eq!(summary(&[("Line".into(), 1)], 1), "Line");
        assert!(summary(&[("Line".into(), 2), ("Arc".into(), 1)], 3).starts_with('3'));
    }
}
