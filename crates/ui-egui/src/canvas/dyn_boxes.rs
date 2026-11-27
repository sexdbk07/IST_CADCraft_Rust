use super::widgets::draw_lock;
use egui::Color32;

use super::{CadApp, Pos2, Rect, Shape, Stroke, Tokens, Vec2, pos2, vec2};

/// The Dynamic Input tooltip beside the cursor, styled like the command line. At a point prompt
/// it shows the pair of value boxes, a view of the command-line text (see [`crate::dyninput`]);
/// otherwise the prompt with what is being typed, or the live distance and angle. With `fields`
/// (a distance prompt's field is on show, [`crate::grip_dyn`]) only the prompt and its options.
pub(super) fn dynamic_input(app: &mut CadApp, p: &egui::Painter, hover: Pos2, fields: bool) {
    app.cmd.option_rows.clear();
    if app.istcad.enabled {
        istcad_dynamic_input(app, p, hover, fields);
        return;
    }
    // While the options menu is open the tooltip stays where it opened, so the menu can be reached.
    let at = match app.cmd.options {
        Some(_) => *app.cmd.options_anchor.get_or_insert(hover),
        None => {
            app.cmd.options_anchor = None;
            hover
        }
    };
    let Some(prompt) = app.session.current_prompt() else { return };
    // DYNPROMPT = 0: no prompt beside the cursor (the value boxes stay).
    let show_prompt = app.session.settings.dynprompt;
    let options = show_prompt && !prompt.keywords.is_empty();
    // An angle about the base (a rotation grip): one angle box, as AutoCAD shows it.
    if !fields
        && prompt.angle
        && crate::cmdline::dyn_frame(app).is_some()
        && let Some(c) = app.canvas.cursor
    {
        let message = if show_prompt { crate::i18n::t(&prompt.message) } else { "" };
        let typed = !app.cmd.buffer.is_empty();
        let value = if typed { app.cmd.buffer.clone() } else { app.session.live_value(c).unwrap_or_default() };
        let r = angle_box(p, at, message, &value, typed, options);
        options_menu(app, p, r, hover);
        return;
    }
    if !fields
        && let Some(f) = crate::cmdline::dyn_frame(app)
        && let Some(e) = crate::dyninput::parse(&app.cmd.buffer, &f)
        && let Some(c) = app.canvas.cursor
    {
        let message = if show_prompt { crate::i18n::t(&prompt.message) } else { "" };
        let r =
            dynamic_input_boxes(p, at, message, &f, &e, c, (app.cmd.first_active(), app.cmd.second_locked()), options, length_units(&app.session));
        options_menu(app, p, r, hover);
        return;
    }
    if !show_prompt {
        return;
    }
    let mut text = crate::i18n::t(&prompt.message).to_string();
    if fields {
        // The field shows the distance and what is typed.
    } else if !app.cmd.buffer.is_empty() {
        text = format!("{}: {}", crate::i18n::t(&prompt.message), app.cmd.buffer);
    } else if let (Some(base), Some(c)) = (prompt.base, app.canvas.cursor) {
        text = format!(
            "{}   {}  <  {}°",
            crate::i18n::t(&prompt.message),
            distance_text(&app.session, base.dist(c)),
            (base.angle_to(c).to_degrees() * 10.0).round() / 10.0
        );
    } else if let Some(c) = app.canvas.cursor {
        text = format!("{}   {}, {}", crate::i18n::t(&prompt.message), distance_text(&app.session, c.x), distance_text(&app.session, c.y));
    }
    let t = Tokens::get();
    let galley = p.layout_no_wrap(text, crate::theme::small(), t.text);
    let hint = if options { OPTIONS_HINT_W } else { 0.0 };
    let r = tooltip_frame(p, at, galley.size() + vec2(12.0 + hint, 8.0));
    p.galley(r.min + vec2(6.0, 4.0), galley, t.text);
    if options {
        options_hint(p, r);
    }
    options_menu(app, p, r, hover);
}

/// The drawing's length units: (LUNITS, LUPREC).
pub(crate) fn length_units(session: &cadcraft_engine::Session) -> (i64, i64) {
    session.doc().map(|d| (d.header.i64("LUNITS", 2), d.header.i64("LUPREC", 4))).unwrap_or((2, 4))
}

/// A distance in the drawing's length units.
fn distance_text(session: &cadcraft_engine::Session, v: f64) -> String {
    let (lu, lp) = length_units(session);
    cadcraft_engine::units::format_distance(v, lu, lp)
}

/// Room for the down-arrow sign that a prompt has options.
const OPTIONS_HINT_W: f32 = 14.0;

/// The down-arrow sign at the right end of the tooltip `r`: the prompt has options, and Down
/// Arrow lists them.
fn options_hint(p: &egui::Painter, r: Rect) {
    let t = Tokens::get();
    let c = pos2(r.right() - OPTIONS_HINT_W / 2.0 - 2.0, r.center().y + 1.0);
    p.add(Shape::convex_polygon(vec![pos2(c.x - 4.0, c.y - 2.5), pos2(c.x + 4.0, c.y - 2.5), pos2(c.x, c.y + 2.5)], t.text_dim, Stroke::NONE));
}

/// The options menu under the tooltip `r` (DYNPROMPT): one row per keyword, the highlighted one
/// marked; the rows are kept for clicks.
fn options_menu(app: &mut CadApp, p: &egui::Painter, r: Rect, hover: Pos2) {
    let Some(mut menu) = app.cmd.options.clone() else { return };
    let t = Tokens::get();
    let font = crate::theme::small();
    let rows: Vec<_> = menu.keywords.iter().map(|k| p.layout_no_wrap(k.clone(), font.clone(), t.text)).collect();
    let row_h = rows.iter().map(|g| g.size().y).fold(12.0, f32::max) + 6.0;
    let w = rows.iter().map(|g| g.size().x).fold(60.0, f32::max) + 28.0;
    let h = row_h * rows.len() as f32 + 4.0;
    let m = Rect::from_min_size(pos2(r.left(), r.bottom() + 2.0), vec2(w, h));
    p.rect_filled(m, 3.0, t.cmd_bg);
    p.rect_stroke(m, 3.0, Stroke::new(1.0, t.cmd_border), egui::StrokeKind::Inside);
    let row_at = |i: usize| Rect::from_min_size(pos2(m.left() + 2.0, m.top() + 2.0 + row_h * i as f32), vec2(w - 4.0, row_h));
    // The row under the pointer is the highlighted one.
    if let Some(i) = (0..menu.keywords.len()).find(|i| row_at(*i).contains(hover)) {
        menu.index = i;
        app.cmd.options = Some(menu.clone());
    }
    for (i, (g, k)) in rows.into_iter().zip(&menu.keywords).enumerate() {
        let row = row_at(i);
        if i == menu.index {
            p.rect_filled(row, 2.0, t.cmd_keyword_hover);
            p.circle_filled(pos2(row.left() + 9.0, row.center().y), 2.5, t.accent);
        }
        p.galley(pos2(row.left() + 20.0, row.center().y - g.size().y / 2.0), g, t.text);
        app.cmd.option_rows.push((row, k.clone()));
    }
}

/// The tooltip background, matching the command line bar; returns its rectangle.
fn tooltip_frame(p: &egui::Painter, at: Pos2, size: egui::Vec2) -> Rect {
    let t = Tokens::get();
    let r = Rect::from_min_size(at + vec2(18.0, -size.y - 14.0), size);
    p.rect_filled(r, 3.0, t.cmd_bg);
    p.rect_stroke(r, 3.0, Stroke::new(1.0, t.cmd_border), egui::StrokeKind::Inside);
    r
}

/// The Dynamic Input tooltip for an angle: the prompt and one box, holding what was typed (full
/// text colour, with the caret) or the live value (dimmed).
fn angle_box(p: &egui::Painter, at: Pos2, message: &str, value: &str, typed: bool, options: bool) -> Rect {
    let t = Tokens::get();
    let font = crate::theme::small();
    let msg = p.layout_no_wrap(message.to_owned(), font.clone(), t.text_dim);
    let val = p.layout_no_wrap(value.to_owned(), font, if typed { t.text } else { t.text_faint });
    let (pad, gap) = (6.0, 6.0);
    let box_h = msg.size().y + 6.0;
    let bw = (val.size().x + 10.0).max(48.0);
    let hint = if options { OPTIONS_HINT_W } else { 0.0 };
    let r = tooltip_frame(p, at, vec2(pad + msg.size().x + gap + bw + pad + hint, box_h + 2.0 * pad - 2.0));
    if options {
        options_hint(p, r);
    }
    let cy = r.center().y;
    let x = r.left() + pad;
    let msg_w = msg.size().x;
    p.galley(pos2(x, cy - msg.size().y / 2.0), msg, t.text_dim);
    let b = Rect::from_min_size(pos2(x + msg_w + gap, cy - box_h / 2.0), vec2(bw, box_h));
    p.rect_filled(b, 2.0, t.chrome_dark);
    p.rect_stroke(b, 2.0, Stroke::new(1.5, t.accent), egui::StrokeKind::Inside);
    let vw = val.size().x;
    p.galley(pos2(b.left() + 5.0, cy - val.size().y / 2.0), val, t.text);
    if typed {
        let cx = b.left() + 6.0 + vw;
        p.line_segment([pos2(cx, b.top() + 3.0), pos2(cx, b.bottom() - 3.0)], Stroke::new(1.0, t.text));
    }
    r
}

/// The Dynamic Input value boxes: X and Y, or distance and angle. Typed values show in full
/// text colour, a locked value (a separator typed after it) gets a lock, the active box gets the
/// accent outline, and open boxes show the cursor's live value, dimmed. Values measured from the
/// last point are marked with `@`. `(first_active, second_locked)`: Tab wrapped around to the first
/// box, and Tab locked the second value.
#[allow(clippy::too_many_arguments)]
fn dynamic_input_boxes(
    p: &egui::Painter,
    at: Pos2,
    message: &str,
    f: &crate::dyninput::Frame,
    e: &crate::dyninput::Entry<'_>,
    cursor: Vec2,
    (first_active, second_locked): (bool, bool),
    options: bool,
    (lu, lp): (i64, i64),
) -> Rect {
    use crate::dyninput::Format;
    let t = Tokens::get();
    let font = crate::theme::small();
    let dist = |v: f64| cadcraft_engine::units::format_distance(v, lu, lp);
    let (live_first, live_second) = f.live(e.format, cursor);
    let live_second = match e.format {
        Format::Cartesian => dist(live_second),
        Format::Polar => format!("{}°", (live_second.to_degrees() * 10.0).round() / 10.0),
    };
    let (label_first, label_second) = match e.format {
        Format::Cartesian => ("X", "Y"),
        Format::Polar => ("", "<"),
    };
    // (label, text, typed, locked, active)
    let cells = [
        (
            label_first,
            if e.first.is_empty() { dist(live_first) } else { e.first.to_owned() },
            !e.first.is_empty(),
            e.first_locked().is_some(),
            !e.split || first_active,
        ),
        (
            label_second,
            if e.second.is_empty() { live_second } else { e.second.to_owned() },
            !e.second.is_empty(),
            second_locked,
            e.split && !first_active,
        ),
    ];

    let msg = p.layout_no_wrap(message.to_owned(), font.clone(), t.text_dim);
    let marker = p.layout_no_wrap(if f.relative() { "@".to_owned() } else { String::new() }, font.clone(), t.text_faint);
    let lock_w = 10.0;
    let laid: Vec<_> = cells
        .into_iter()
        .map(|(label, text, typed, locked, active)| {
            let label = p.layout_no_wrap(label.to_owned(), font.clone(), t.text_faint);
            let value = p.layout_no_wrap(text, font.clone(), if typed { t.text } else { t.text_faint });
            let w = (value.size().x + 10.0 + if locked { lock_w } else { 0.0 }).max(48.0);
            (label, value, w, locked, active)
        })
        .collect();

    // Measure, then draw.
    let pad = 6.0;
    let gap = 6.0;
    let box_h = msg.size().y + 6.0;
    let mut w = pad + msg.size().x + gap + marker.size().x;
    for (label, _, bw, _, _) in &laid {
        w += gap + label.size().x + 3.0 + bw;
    }
    let hint = if options { OPTIONS_HINT_W } else { 0.0 };
    let r = tooltip_frame(p, at, vec2(w + pad + hint, box_h + 2.0 * pad - 2.0));
    if options {
        options_hint(p, r);
    }
    let cy = r.center().y;
    let mut x = r.left() + pad;
    let (msg_w, msg_h) = (msg.size().x, msg.size().y);
    p.galley(pos2(x, cy - msg_h / 2.0), msg, t.text_dim);
    x += msg_w + gap;
    let mw = marker.size().x;
    p.galley(pos2(x, cy - marker.size().y / 2.0), marker, t.text_faint);
    x += mw;
    for (label, value, bw, locked, active) in laid {
        x += gap;
        let lw = label.size().x;
        p.galley(pos2(x, cy - label.size().y / 2.0), label, t.text_faint);
        x += lw + 3.0;
        let b = Rect::from_min_size(pos2(x, cy - box_h / 2.0), vec2(bw, box_h));
        p.rect_filled(b, 2.0, t.chrome_dark);
        let stroke = if active { Stroke::new(1.5, t.accent) } else { Stroke::new(1.0, t.border) };
        p.rect_stroke(b, 2.0, stroke, egui::StrokeKind::Inside);
        let vw = value.size().x;
        p.galley(pos2(b.left() + 5.0, cy - value.size().y / 2.0), value, t.text);
        if active {
            let cx = b.left() + 6.0 + vw;
            p.line_segment([pos2(cx, b.top() + 3.0), pos2(cx, b.bottom() - 3.0)], Stroke::new(1.0, t.text));
        }
        if locked {
            draw_lock(p, pos2(b.right() - lock_w / 2.0 - 3.0, cy), t.accent);
        }
        x += bw;
    }
    r
}

/// iStCAD's Dynamic Input (1.0.231): two separate pieces. The prompt follows the pointer (18, 21)
/// px below and right of it as a light, non-interactive hint; the editable box, a `↳` and one
/// field, appears only once something is typed (18, 38) px from the pointer and stays where it
/// opened while typing. No live distance or angle fields.
fn istcad_dynamic_input(app: &mut CadApp, p: &egui::Painter, hover: Pos2, fields: bool) {
    let typed = app.cmd.buffer.clone();
    if typed.is_empty() {
        app.istcad.dyn_box = None;
    }
    let at = match app.cmd.options {
        Some(_) => *app.cmd.options_anchor.get_or_insert(hover),
        None => {
            app.cmd.options_anchor = None;
            hover
        }
    };
    let hint = match app.session.current_prompt() {
        Some(pr) if app.session.settings.dynprompt => {
            let mut m = if pr.raw { pr.display() } else { crate::i18n::t(&pr.message).to_string() };
            if !pr.raw && !pr.keywords.is_empty() {
                m.push_str(" [");
                m.push_str(&pr.keywords.join("/"));
                m.push(']');
            }
            if let Some(d) = pr.default.as_ref().filter(|_| !pr.raw) {
                m.push_str(" <");
                m.push_str(d);
                m.push('>');
            }
            if !m.trim_end().ends_with(':') {
                m.push(':');
            }
            Some(prompt_hint(p, at, &m))
        }
        _ => None,
    };
    if !typed.is_empty() && !fields {
        let area = p.clip_rect();
        let r = *app.istcad.dyn_box.get_or_insert_with(|| {
            let size = vec2(ENTRY_W, ENTRY_H);
            let left = (at.x + 18.0).clamp(area.left() + 6.0, (area.right() - size.x - 6.0).max(area.left() + 6.0));
            let top = (at.y + 38.0).clamp(area.top() + 6.0, (area.bottom() - size.y - 6.0).max(area.top() + 6.0));
            Rect::from_min_size(pos2(left, top).round(), size)
        });
        entry(p, r, &typed);
    }
    if let Some(r) = hint {
        options_menu(app, p, r, hover);
    }
}

/// The editable box's size: iStCAD's 162 px panel, 23 px high.
const ENTRY_W: f32 = 162.0;
const ENTRY_H: f32 = 23.0;

/// The prompt hint: 10 pt `#D5E1E6` on `#1E2B31` at 73 %, corner 3, padding 5 × 2, at most 245 px
/// wide (longer text ends in an ellipsis), kept inside the drawing.
fn prompt_hint(p: &egui::Painter, at: Pos2, text: &str) -> Rect {
    const FILL: Color32 = Color32::from_rgba_unmultiplied_const(0x1e, 0x2b, 0x31, 0xba);
    const INK: Color32 = Color32::from_rgb(0xd5, 0xe1, 0xe6);
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), egui::FontId::proportional(10.0), INK);
    job.wrap = egui::text::TextWrapping::truncate_at_width(245.0);
    let g = p.layout_job(job);
    let size = vec2(g.size().x + 10.0, g.size().y + 4.0);
    let area = p.clip_rect();
    let left = (at.x + 18.0).clamp(area.left() + 6.0, (area.right() - size.x - 6.0).max(area.left() + 6.0));
    let top = (at.y + 21.0).clamp(area.top() + 6.0, (area.bottom() - size.y - 6.0).max(area.top() + 6.0));
    let r = Rect::from_min_size(pos2(left, top).round(), size);
    p.rect_filled(r, 3.0, FILL);
    p.galley(pos2(r.left() + 5.0, r.top() + 2.0), g, INK);
    r
}

/// The editable box in `r`: a `↳` (`#C0C5CE`) and a 143 × 23 field, `#4A515B` with a
/// `#858B94` border, the typed text white at 11 pt with the caret.
fn entry(p: &egui::Painter, r: Rect, text: &str) {
    const FIELD: Color32 = Color32::from_rgb(0x4a, 0x51, 0x5b);
    const EDGE: Color32 = Color32::from_rgb(0x85, 0x8b, 0x94);
    const ARROW: Color32 = Color32::from_rgb(0xc0, 0xc5, 0xce);
    let cy = r.center().y;
    // The `↳` drawn as strokes, so it shows whatever fonts are installed.
    let ink = Stroke::new(1.2, ARROW);
    let (x0, x1) = (r.left() + 3.0, r.left() + 13.0);
    p.line_segment([pos2(x0, cy - 6.0), pos2(x0, cy + 2.0)], ink);
    p.line_segment([pos2(x0, cy + 2.0), pos2(x1, cy + 2.0)], ink);
    p.line_segment([pos2(x1 - 3.5, cy - 1.5), pos2(x1, cy + 2.0)], ink);
    p.line_segment([pos2(x1 - 3.5, cy + 5.5), pos2(x1, cy + 2.0)], ink);
    let b = Rect::from_min_size(pos2(r.right() - 143.0, r.top()), vec2(143.0, ENTRY_H));
    p.rect(b, 0.0, FIELD, Stroke::new(1.0, EDGE), egui::StrokeKind::Inside);
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), egui::FontId::proportional(11.0), Color32::WHITE);
    job.wrap = egui::text::TextWrapping::truncate_at_width(b.width() - 10.0);
    let g = p.layout_job(job);
    let cx = (b.left() + 5.0 + g.size().x + 1.0).min(b.right() - 3.0);
    p.galley(pos2(b.left() + 5.0, cy - g.size().y / 2.0), g, Color32::WHITE);
    p.line_segment([pos2(cx, b.top() + 5.0), pos2(cx, b.bottom() - 5.0)], Stroke::new(1.0, Color32::WHITE));
}
