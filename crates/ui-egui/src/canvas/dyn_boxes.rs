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

/// iStCAD's dynamic input panel (`ViewportDynamicInputPanel`): 242 points wide, 18 points below
/// and right of the pointer and kept 8 points inside the drawing area; `#363C45` at 95 %, a
/// `#AEB4BE` border, radius 3, padding 5,4. The prompt (10 points, `#F0F1F4`) over "↳" and the
/// value field (23 high, `#4A515B`, `#858B94` border, 11 points, white). The field holds what is
/// typed; with nothing typed it shows the live value under the pointer, dimmed. With no command
/// running, typing shows "Command:" and the command name, as iStCAD's viewport entry does. Typing,
/// Tab, the options menu (↓) and Enter keep the command line's handling.
fn istcad_dynamic_input(app: &mut CadApp, p: &egui::Painter, hover: Pos2, fields: bool) {
    const W: f32 = 242.0;
    const FILL: Color32 = Color32::from_rgba_unmultiplied_const(0x36, 0x3c, 0x45, 0xf2);
    const EDGE: Color32 = Color32::from_rgb(0xae, 0xb4, 0xbe);
    const PROMPT: Color32 = Color32::from_rgb(0xf0, 0xf1, 0xf4);
    const ARROW: Color32 = Color32::from_rgb(0xc0, 0xc5, 0xce);
    const FIELD: Color32 = Color32::from_rgb(0x4a, 0x51, 0x5b);
    const FIELD_EDGE: Color32 = Color32::from_rgb(0x85, 0x8b, 0x94);
    let prompt = app.session.current_prompt();
    let typed = app.cmd.buffer.clone();
    let (message, options) = match &prompt {
        Some(pr) if app.session.settings.dynprompt => {
            let mut m = if pr.raw { pr.display() } else { crate::i18n::t(&pr.message).to_string() };
            if !pr.raw && !pr.keywords.is_empty() {
                m.push_str(&format!(" [{}]", pr.keywords.join("/")));
            }
            if let Some(d) = pr.default.as_ref().filter(|_| !pr.raw) {
                m.push_str(&format!(" <{d}>"));
            }
            if !pr.raw {
                m.push(':');
            }
            (m, !pr.keywords.is_empty())
        }
        Some(_) => (String::new(), false),
        // No command: the panel shows only while a command name is being typed.
        None if !typed.is_empty() => (crate::tl!("Command:").to_string(), false),
        None => return,
    };
    // With nothing typed, the live value under the pointer (dimmed), as a hint of what Enter takes.
    let live = if !typed.is_empty() || fields || prompt.is_none() {
        None
    } else {
        app.canvas.cursor.and_then(|c| {
            app.session.live_value(c).or_else(|| match prompt.as_ref().and_then(|pr| pr.base) {
                Some(base) => {
                    Some(format!("{} < {}°", distance_text(&app.session, base.dist(c)), (base.angle_to(c).to_degrees() * 10.0).round() / 10.0))
                }
                None => prompt
                    .as_ref()
                    .filter(|pr| pr.accept.point)
                    .map(|_| format!("{}, {}", distance_text(&app.session, c.x), distance_text(&app.session, c.y))),
            })
        })
    };
    let at = match app.cmd.options {
        Some(_) => *app.cmd.options_anchor.get_or_insert(hover),
        None => {
            app.cmd.options_anchor = None;
            hover
        }
    };
    let area = p.clip_rect();
    let show_prompt = !message.is_empty();
    let h = 4.0 + if show_prompt { 13.0 + 3.0 } else { 0.0 } + 23.0 + 4.0;
    let left = (at.x + 18.0).clamp(area.left() + 8.0, (area.right() - W - 8.0).max(area.left() + 8.0));
    let top = (at.y + 18.0).clamp(area.top() + 8.0, (area.bottom() - h - 8.0).max(area.top() + 8.0));
    let r = Rect::from_min_size(pos2(left, top), vec2(W, h));
    p.rect(r, 3.0, FILL, Stroke::new(1.0, EDGE), egui::StrokeKind::Inside);
    let inner = r.shrink2(vec2(5.0, 4.0));
    let mut y = inner.top();
    if show_prompt {
        let mut job = egui::text::LayoutJob::simple_singleline(message, egui::FontId::proportional(10.0), PROMPT);
        let room = inner.width() - if options { OPTIONS_HINT_W } else { 0.0 };
        job.wrap = egui::text::TextWrapping { max_width: room, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
        let g = p.layout_job(job);
        p.galley(pos2(inner.left(), y), g, PROMPT);
        if options {
            options_hint(p, Rect::from_min_max(pos2(r.left(), r.top()), pos2(r.right(), r.top() + 16.0 + 4.0)));
        }
        y += 13.0 + 3.0;
    }
    let row_cy = y + 23.0 / 2.0;
    p.text(pos2(inner.left(), row_cy), egui::Align2::LEFT_CENTER, "↳", egui::FontId::proportional(14.0), ARROW);
    let field = Rect::from_min_max(pos2(inner.left() + 18.0, y), pos2(inner.right(), y + 23.0));
    p.rect(field, 2.0, FIELD, Stroke::new(1.0, FIELD_EDGE), egui::StrokeKind::Inside);
    let font = egui::FontId::proportional(11.0);
    let mut x = field.left() + 4.0;
    if typed.is_empty() {
        if let Some(live) = live {
            let mut job = egui::text::LayoutJob::simple_singleline(live, font, FIELD_EDGE);
            job.wrap =
                egui::text::TextWrapping { max_width: field.width() - 10.0, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
            p.galley(pos2(x, row_cy - 7.0), p.layout_job(job), FIELD_EDGE);
        }
    } else {
        let g = p.layout_no_wrap(typed, font, Color32::WHITE);
        // A long entry shows its end, where the caret is.
        let shift = (g.size().x - (field.width() - 12.0)).max(0.0);
        let gw = g.size().x;
        p.with_clip_rect(field.shrink(1.0)).galley(pos2(x - shift, row_cy - g.size().y / 2.0), g, Color32::WHITE);
        x += gw - shift;
    }
    if !app.cmd.preedit.is_empty() {
        let g = p.layout_no_wrap(app.cmd.preedit.clone(), egui::FontId::proportional(11.0), Color32::WHITE);
        let gw = g.size().x;
        p.galley(pos2(x, row_cy - g.size().y / 2.0), g, Color32::WHITE);
        p.line_segment([pos2(x, row_cy + 7.0), pos2(x + gw, row_cy + 7.0)], Stroke::new(1.0, EDGE));
        x += gw;
    }
    if !fields {
        p.line_segment([pos2(x + 1.0, row_cy - 7.0), pos2(x + 1.0, row_cy + 7.0)], Stroke::new(1.0, Color32::WHITE));
    }
    if app.cmd.second_locked() {
        draw_lock(p, pos2(field.right() - 9.0, row_cy), EDGE);
    }
    options_menu(app, p, r, hover);
}
