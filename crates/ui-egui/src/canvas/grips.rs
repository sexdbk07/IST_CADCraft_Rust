use super::{CadApp, Handle, Input, Pos2, Rect, Xf, vec2};

/// A click on grip `g` (idle): Shift+click makes it hot or cold again; a plain click starts
/// editing it, with the other hot grips when it is one of them.
pub(super) fn click_grip(app: &mut CadApp, g: (Handle, usize), shift: bool) {
    // A hot grip whose object was hidden since (a layer turned off, HIDEOBJECTS, a layer frozen
    // in the active viewport) is no longer painted, so it must not be stretched along with the
    // visible ones.
    let Ok(st) = app.session.state() else { return };
    app.canvas.hot_grips.retain(|(h, _)| st.doc.entity(*h).is_some_and(|e| st.shows(e)));
    let hot = &mut app.canvas.hot_grips;
    if shift {
        match hot.iter().position(|x| *x == g) {
            Some(i) => {
                hot.remove(i);
            }
            None => hot.push(g),
        }
        return;
    }
    if !hot.contains(&g) {
        hot.clear();
    }
    hot.retain(|x| *x != g);
    hot.insert(0, g);
    let hs = hot.clone();
    if let Err(e) = app.session.start_grip_edit(&hs) {
        app.canvas.hot_grips.clear();
        app.session.echo(e.to_string());
    }
}

/// The multifunctional options of grip `g` (Add Vertex, Lengthen…; empty: Stretch only).
pub(super) fn grip_options(app: &CadApp, (h, i): (Handle, usize)) -> Vec<cadcraft_engine::grips::GripAction> {
    let Ok(st) = app.session.state() else { return Vec::new() };
    st.doc.entity(h).map(|e| cadcraft_engine::grips::grip_actions(&e.kind, i)).unwrap_or_default()
}

/// The menu of a multifunctional grip: Stretch and its options. A choice makes the grip hot and
/// runs the option at the grip prompt, as typing its keyword would.
pub(super) fn grip_menu(app: &mut CadApp, resp: &egui::Response) {
    let Some(g) = app.canvas.grip_menu else { return };
    let options = grip_options(app, g);
    let mut chosen: Option<Option<cadcraft_engine::grips::GripAction>> = None;
    let open = resp
        .context_menu(|ui| {
            let vertex = options.contains(&cadcraft_engine::grips::GripAction::RemoveVertex);
            if ui.button(crate::i18n::t(if vertex { "Stretch Vertex" } else { "Stretch" })).clicked() {
                chosen = Some(None);
                ui.close();
            }
            for a in &options {
                if ui.button(crate::i18n::t(a.label())).clicked() {
                    chosen = Some(Some(*a));
                    ui.close();
                }
            }
        })
        .is_some();
    if !open || chosen.is_some() {
        app.canvas.grip_menu = None;
    }
    let Some(choice) = chosen else { return };
    app.canvas.hot_grips = vec![g];
    let r = app.session.start_grip_edit(&[g]).and_then(|()| match choice {
        Some(a) => app.session.input(Input::Keyword(a.keyword().to_string())),
        None => Ok(()),
    });
    if let Err(e) = r {
        app.canvas.hot_grips.clear();
        app.session.echo(e.to_string());
    }
}

/// How long the pointer rests on a multifunctional grip before its options show, in seconds.
const GRIP_HOVER_DELAY: f64 = 0.35;

/// AutoCAD's hover menu of a multifunctional grip: after the pointer rests on a grip with options
/// (a polyline vertex, a line's end…), Stretch and the options show beside it; a click runs one
/// as the right-click menu does. It stays while the pointer moves onto it.
pub(super) fn grip_hover_menu(app: &mut CadApp, ctx: &egui::Context, xf: &Xf, hover: Option<Pos2>) {
    let now = ctx.input(|i| i.time);
    let under = grip_at(app, xf, hover).filter(|g| !grip_options(app, *g).is_empty());
    let on_menu = |r: Option<Rect>| r.zip(hover).is_some_and(|(r, p)| r.expand(4.0).contains(p));
    app.canvas.grip_hover = match (app.canvas.grip_hover.take(), under) {
        (Some((g, since, r)), Some(u)) if g == u => Some((g, since, r)),
        (Some((g, since, r)), _) if r.is_some() && on_menu(r) => Some((g, since, r)),
        (_, Some(u)) => Some((u, now, None)),
        (_, None) => None,
    };
    let Some((g, since, _)) = app.canvas.grip_hover else { return };
    if now - since < GRIP_HOVER_DELAY {
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(GRIP_HOVER_DELAY - (now - since) + 0.01));
        return;
    }
    let Some(at) = app.session.state().ok().and_then(|st| st.doc.entity(g.0).and_then(|e| e.kind.grips().get(g.1).copied())).map(|p| xf.to_screen(p)) else {
        app.canvas.grip_hover = None;
        return;
    };
    let options = grip_options(app, g);
    let vertex = options.contains(&cadcraft_engine::grips::GripAction::RemoveVertex);
    let mut chosen: Option<Option<cadcraft_engine::grips::GripAction>> = None;
    let shown = egui::Area::new(egui::Id::unique("grip_hover_menu")).order(egui::Order::Foreground).fixed_pos(at + vec2(10.0, 10.0)).show(ctx, |ui| {
        egui::Frame::menu(ui.style()).show(ui, |ui| {
            ui.set_min_width(130.0);
            if ui.button(crate::i18n::t(if vertex { "Stretch Vertex" } else { "Stretch" })).clicked() {
                chosen = Some(None);
            }
            for a in &options {
                if ui.button(crate::i18n::t(a.label())).clicked() {
                    chosen = Some(Some(*a));
                }
            }
        });
    });
    if let Some((_, _, r)) = &mut app.canvas.grip_hover {
        *r = Some(shown.response.rect);
    }
    let Some(choice) = chosen else { return };
    app.canvas.grip_hover = None;
    app.canvas.hot_grips = vec![g];
    let r = app.session.start_grip_edit(&[g]).and_then(|()| match choice {
        Some(a) => app.session.input(Input::Keyword(a.keyword().to_string())),
        None => Ok(()),
    });
    if let Err(e) = r {
        app.canvas.hot_grips.clear();
        app.session.echo(e.to_string());
    }
}

/// Polyline segment midpoint grips are rectangular; vertex grips remain square.
pub(super) fn grip_rect(kind: &cadcraft_doc::EntityKind, count: usize, index: usize, center: Pos2, size: f32) -> Rect {
    // The grip list interleaves vertices and available segment midpoints, then any remaining vertices.
    let midpoint = matches!(kind, cadcraft_doc::EntityKind::LwPolyline(p) if index % 2 == 1 && index / 2 < count.saturating_sub(p.vertices.len()));
    Rect::from_center_size(center, vec2(size * 2.0, if midpoint { size } else { size * 2.0 }))
}

/// The grip of a selected object under the cursor, if any: (object, grip index).
pub(super) fn grip_at(app: &CadApp, xf: &Xf, hover: Option<Pos2>) -> Option<(Handle, usize)> {
    let hp = hover?;
    let st = app.session.state().ok()?;
    let gs = app.session.settings.gripsize as f32;
    let sel = app.session.selection();
    if !app.session.shows_grips(sel.len()) {
        return None;
    }
    for h in sel.iter().take(200) {
        let e = st.doc.entity(*h)?;
        if !st.shows(e) || st.doc.layer(&e.common.layer).is_some_and(|l| l.locked) {
            continue;
        }
        let grips = e.kind.grips();
        let count = grips.len();
        for (i, g) in grips.into_iter().enumerate() {
            let sp = xf.to_screen(g);
            if grip_rect(&e.kind, count, i, sp, gs).expand(3.0).contains(hp) {
                return Some((*h, i));
            }
        }
    }
    None
}
