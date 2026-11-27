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

/// AutoCAD's Ctrl while a single hot grip is being edited: each press moves to the grip's next
/// option (Stretch, then Add Vertex, Remove Vertex… then Stretch again), as the option's keyword
/// would.
pub(super) fn ctrl_cycles_options(app: &mut CadApp, ctx: &egui::Context, grip_edit: bool) {
    let down = ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt);
    let pressed = down && !app.canvas.ctrl_down;
    app.canvas.ctrl_down = down;
    // Ctrl with a key is a shortcut, not a cycle.
    if !pressed || !grip_edit || ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Key { pressed: true, .. }))) {
        return;
    }
    let [g] = app.canvas.hot_grips.as_slice() else { return };
    let options = grip_options(app, *g);
    if options.is_empty() {
        return;
    }
    let message = app.session.current_prompt().map(|p| p.message).unwrap_or_default();
    // The option being worked, from the prompt's title; none: Stretch.
    let current = options.iter().position(|a| message.starts_with(a.title()));
    if current.is_none() && !message.starts_with("** STRETCH **") {
        return;
    }
    let next = current.map_or(0, |i| i + 1);
    // From an option, Enter goes back to Stretch first.
    if current.is_some()
        && let Err(e) = app.session.input(Input::Enter)
    {
        app.session.echo(e.to_string());
        return;
    }
    if let Some(a) = options.get(next)
        && let Err(e) = app.session.input(Input::Keyword(a.keyword().to_string()))
    {
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

/// The groups shown as one (GROUPDISPLAYMODE 1 or 2, with PICKSTYLE's group selection): each
/// selectable group whose members are all selected, its members and their box.
pub(super) fn selected_groups(app: &CadApp) -> Vec<(Vec<Handle>, cadcraft_geom::Bounds2)> {
    let s = &app.session.settings;
    if s.groupdisplaymode == 0 || s.pickstyle & 1 == 0 {
        return Vec::new();
    }
    let Ok(st) = app.session.state() else { return Vec::new() };
    let sel = app.session.selection();
    if sel.is_empty() {
        return Vec::new();
    }
    st.doc
        .groups
        .iter()
        .filter(|g| g.selectable)
        .filter_map(|g| {
            let members: Vec<Handle> = g.members.iter().copied().filter(|m| st.doc.entity(*m).is_some()).collect();
            if members.is_empty() || !members.iter().all(|m| sel.contains(m)) {
                return None;
            }
            let b = members
                .iter()
                .filter_map(|m| st.doc.entity(*m))
                .map(|e| cadcraft_doc::entity_bounds(&st.doc, e, 0))
                .filter(|b| !b.is_empty())
                .fold(cadcraft_geom::Bounds2::EMPTY, |a, b| a.union(&b));
            (!b.is_empty()).then_some((members, b))
        })
        .collect()
}

/// The centre grip of a selected group under the cursor: the group's members and its centre.
pub(super) fn group_grip_at(app: &CadApp, xf: &Xf, hover: Option<Pos2>) -> Option<(Vec<Handle>, cadcraft_geom::Vec2)> {
    let hp = hover?;
    let gs = app.session.settings.gripsize as f32;
    selected_groups(app).into_iter().find_map(|(members, b)| {
        let c = b.center();
        Rect::from_center_size(xf.to_screen(c), vec2(gs * 2.0, gs * 2.0)).expand(3.0).contains(hp).then_some((members, c))
    })
}

/// A group's centre grip clicked: MOVE its members from the centre, as AutoCAD stretches a
/// group's grip.
pub(super) fn click_group_grip(app: &mut CadApp, members: Vec<Handle>, centre: cadcraft_geom::Vec2) {
    app.session.set_selection(members);
    app.start("move");
    if app.session.running.as_ref().is_some_and(|r| r.id == "move")
        && let Err(e) = app.session.input(Input::Point(centre))
    {
        app.session.echo(e.to_string());
    }
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
    let grouped: Vec<Handle> = selected_groups(app).into_iter().flat_map(|(m, _)| m).collect();
    for h in sel.iter().take(200) {
        let e = st.doc.entity(*h)?;
        if !st.shows(e) || st.doc.layer(&e.common.layer).is_some_and(|l| l.locked) || grouped.contains(h) {
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
