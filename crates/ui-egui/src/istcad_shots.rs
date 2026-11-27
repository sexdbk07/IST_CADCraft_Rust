//! The iStCAD window drawn whole, as the desktop app opens it: the rows and palettes it must
//! show. With `CRAFT_UI_SUITE_SHOTS=<folder>` the frames are rendered (wgpu) and saved as PNG
//! there, so the window can be compared with iStCAD's layout (`docs/ISTCAD.md`) without a display.

use crate::{CadApp, Services};
use cadcraft_engine::Session;
use egui::{Shape, pos2, vec2};
use egui_kittest::Harness;
use serde_json::json;

fn texts(shape: &Shape, out: &mut Vec<String>) {
    match shape {
        Shape::Text(t) => out.push(t.galley.text().to_string()),
        Shape::Vec(shapes) => shapes.iter().for_each(|shape| texts(shape, out)),
        _ => {}
    }
}

fn painted(h: &Harness<'_, CadApp>) -> Vec<String> {
    let mut out = Vec::new();
    for shape in &h.output().shapes {
        texts(&shape.shape, &mut out);
    }
    out
}

/// Where the text `label` is painted.
fn text_pos(h: &Harness<'_, CadApp>, label: &str) -> Option<egui::Pos2> {
    fn find(shape: &Shape, label: &str) -> Option<egui::Pos2> {
        match shape {
            Shape::Text(t) if t.galley.text() == label => Some(t.pos),
            Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, label)),
            _ => None,
        }
    }
    h.output().shapes.iter().find_map(|s| find(&s.shape, label))
}

/// A primary click at `at`.
fn click(h: &mut Harness<'_, CadApp>, at: egui::Pos2) {
    h.event(egui::Event::PointerMoved(at));
    h.run_steps(1);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE });
    }
    h.run_steps(1);
}

/// The app as `apps/cadcraft` starts it with the iStCAD shell, on the sample drawing.
fn istcad_app() -> CadApp {
    let mut app = CadApp::new(Session::default(), Services::default());
    app.istcad.enabled = true;
    app.istcad.custom_frame = true;
    app.ui.show_toolsets = false;
    app.ui.show_palettes = false;
    app.ui.show_viewcube = false;
    app.ui.in_window_menu = false;
    app.session.settings.gridmode = true;
    let _ = app.run("ui.sample", json!({}));
    app
}

#[test]
fn istcad_window_shows_its_rows_and_properties_palette() {
    let shots = std::env::var_os("CRAFT_UI_SUITE_SHOTS").map(std::path::PathBuf::from);
    let mut builder = Harness::builder().with_size(vec2(1600.0, 900.0));
    if shots.is_some() {
        builder = builder.wgpu();
    }
    let mut h = builder.build_ui_state(
        |ui, app: &mut CadApp| {
            app.logic(&ui.ctx().clone());
            app.ui(ui);
        },
        istcad_app(),
    );
    let save = |h: &mut Harness<'_, CadApp>, name: &str| {
        if let Some(dir) = &shots {
            std::fs::create_dir_all(dir).unwrap();
            h.render().unwrap().save(dir.join(format!("{name}.png"))).unwrap();
        }
    };
    h.run_steps(6);
    let shown = painted(&h);
    for label in ["Text Style", "Dim Style", "Layer", "Color", "Line Style", "Command:", "Model"] {
        assert!(shown.iter().any(|t| t.contains(label)), "the iStCAD window doesn't show {label:?}: {shown:?}");
    }
    // CADCraft's menus are in the title bar, folded into seven: no second menu row, and the
    // folded menus (Window, Dimension, Insert, Format) are submenus, not buttons. No viewport label.
    for menu in ["File", "Edit", "View", "Draw", "Modify", "Tools", "Help"] {
        assert_eq!(shown.iter().filter(|t| t.as_str() == menu).count(), 1, "{menu} in {shown:?}");
    }
    for folded in ["Window", "Dimension", "Insert", "Format"] {
        assert!(!shown.iter().any(|t| t == folded), "{folded} is a button");
    }
    assert!(!shown.iter().any(|t| t.contains("2D Wireframe")), "the iStCAD window shows a viewport label");
    save(&mut h, "01-start");

    // Ctrl+1 with everything selected: the Properties palette on the right.
    let _ = h.state_mut().run("selectall", json!({}));
    let _ = h.state_mut().run("ui.istcad.properties", json!({ "on": true }));
    h.run_steps(4);
    let shown = painted(&h);
    for group in ["PROPERTIES", "General", "Misc"] {
        assert!(shown.iter().any(|t| t == group), "Ctrl+1 shows the Properties palette with {group:?}: {shown:?}");
    }
    save(&mut h, "02-properties");

    // One circle: its Geometry rows, edited through the palette's own path.
    let circle = h.state().session.doc().ok().and_then(|d| {
        d.model.iter().find(|e| matches!(e.kind, cadcraft_engine::doc::EntityKind::Circle(_))).map(|e| e.handle.hex())
    });
    if let Some(c) = circle {
        let _ = h.state_mut().run("select", json!({ "clear": true }));
        let _ = h.state_mut().run("select", json!({ "handles": [c] }));
        h.run_steps(4);
        let shown = painted(&h);
        for label in ["Geometry", "Center X", "Radius", "Diameter"] {
            assert!(shown.iter().any(|t| t == label), "a circle's Properties show {label:?}: {shown:?}");
        }
        save(&mut h, "05-circle-properties");
    }
    let _ = h.state_mut().run("ui.istcad.properties", json!({ "on": false }));
    let _ = h.state_mut().run("select", json!({ "clear": true }));

    // A command name being typed: AutoComplete above the command bar.
    h.state_mut().cmd.buffer = "LI".into();
    h.run_steps(3);
    save(&mut h, "03-autocomplete");
    h.state_mut().cmd.buffer.clear();

    // LINE running, the pointer over the drawing: the prompt and the dynamic input box.
    h.state_mut().start("line");
    h.event(egui::Event::PointerMoved(pos2(760.0, 430.0)));
    h.run_steps(3);
    save(&mut h, "04-line-dyn");

    // After the first point: the command tooltip apart from the input fields, the distance on
    // the rubber band and the angle by the pointer.
    click(&mut h, pos2(700.0, 450.0));
    h.event(egui::Event::PointerMoved(pos2(860.0, 380.0)));
    h.run_steps(3);
    let shown = painted(&h);
    assert!(shown.iter().any(|t| t.starts_with("Specify next point")), "the command tooltip: {shown:?}");
    assert!(shown.iter().any(|t| t == "<"), "the angle field: {shown:?}");
    save(&mut h, "06-line-dyn-polar");
    h.key_press(egui::Key::Escape);
    h.run_steps(2);

    // PLOT: iStCAD's Plot (PDF) dialog with its live preview.
    cadcraft_engine::cmd::file::set_io(cadcraft_engine::cmd::file::IoHooks {
        read: |b, name| cadcraft_io::read(b, name).map_err(|e| e.to_string()),
        write: |d, name| cadcraft_io::write(d, name).map_err(|e| e.to_string()),
        plot: Some(|d, space, opts| cadcraft_io::plotter::plot(d, space, opts).map_err(|e| e.to_string())),
    });
    h.state_mut().start("plot");
    h.run_steps(4);
    let shown = painted(&h);
    assert!(shown.iter().any(|t| t == "Plot PDF") && shown.iter().any(|t| t == "PAPER"), "the Plot (PDF) dialog: {shown:?}");
    save(&mut h, "07-plot");
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
}

/// The palette edits by hand, as a user does: click a value, type, Enter.
#[test]
fn istcad_properties_values_are_edited_by_typing() {
    let mut h = Harness::builder().with_size(vec2(1600.0, 900.0)).build_ui_state(
        |ui, app: &mut CadApp| {
            app.logic(&ui.ctx().clone());
            app.ui(ui);
        },
        istcad_app(),
    );
    h.run_steps(3);
    let radius = |h: &Harness<'_, CadApp>, hex: &str| {
        let d = h.state().session.doc().unwrap();
        d.model.iter().find(|e| e.handle.hex() == hex).and_then(|e| match &e.kind {
            cadcraft_engine::doc::EntityKind::Circle(c) => Some(c.radius),
            _ => None,
        })
    };
    let circle = h.state().session.doc().ok().and_then(|d| {
        d.model.iter().find(|e| matches!(e.kind, cadcraft_engine::doc::EntityKind::Circle(_))).map(|e| e.handle.hex())
    });
    let Some(c) = circle else { panic!("the sample has a circle") };
    let _ = h.state_mut().run("select", json!({ "handles": [c] }));
    let _ = h.state_mut().run("ui.istcad.properties", json!({ "on": true }));
    h.run_steps(4);
    let before = radius(&h, &c).unwrap();
    // Click the Radius row where it is painted.
    let at = text_pos(&h, "Radius").expect("the Radius row is painted");
    click(&mut h, at + vec2(150.0, 6.0));
    h.run_steps(3);
    // The field opens with the current value selected: typing replaces it.
    h.event(egui::Event::Text("2.5".into()));
    h.run_steps(1);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    let after = radius(&h, &c).unwrap();
    assert!((after - 2.5).abs() < 1e-9, "Radius {before} → {after}: typing 2.5 and Enter changes it");
}

/// MText contents open in a box of several lines: Enter breaks a line, Ctrl+Enter applies.
#[test]
fn istcad_properties_contents_take_several_lines() {
    let mut h = Harness::builder().with_size(vec2(1600.0, 900.0)).build_ui_state(
        |ui, app: &mut CadApp| {
            app.logic(&ui.ctx().clone());
            app.ui(ui);
        },
        istcad_app(),
    );
    h.run_steps(3);
    let mtext = h.state().session.doc().ok().and_then(|d| {
        d.model.iter().find(|e| matches!(e.kind, cadcraft_engine::doc::EntityKind::MText(_))).map(|e| e.handle.hex())
    });
    let Some(m) = mtext else { panic!("the sample has mtext") };
    let contents = |h: &Harness<'_, CadApp>| {
        let d = h.state().session.doc().unwrap();
        d.model.iter().find(|e| e.handle.hex() == m).and_then(|e| match &e.kind {
            cadcraft_engine::doc::EntityKind::MText(t) => Some(t.contents.clone()),
            _ => None,
        })
    };
    let _ = h.state_mut().run("select", json!({ "handles": [m] }));
    let _ = h.state_mut().run("ui.istcad.properties", json!({ "on": true }));
    h.run_steps(4);
    let at = text_pos(&h, "Contents").expect("the Contents row is painted");
    click(&mut h, at + vec2(150.0, 6.0));
    h.run_steps(3);
    h.event(egui::Event::Text("ONE".into()));
    h.key_press(egui::Key::Enter);
    h.event(egui::Event::Text("TWO".into()));
    h.run_steps(1);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
    h.run_steps(3);
    assert_eq!(contents(&h).as_deref(), Some("ONE\\PTWO"));
}

/// A block reference's attributes are listed by tag and edited in place.
#[test]
fn istcad_properties_edit_block_attributes() {
    let mut app = istcad_app();
    let _ = app.run("new", json!({}));
    app.run("attdef", json!({ "tag": "PARTNO", "default": "A-1", "at": [0, 1] })).unwrap();
    app.run("line", json!({ "points": [[0, 0], [10, 0]] })).unwrap();
    let hs: Vec<String> = app.session.doc().unwrap().model.iter().map(|e| e.handle.hex()).collect();
    app.run("block", json!({ "name": "TAGGED", "base": [0, 0], "handles": hs, "keep": "delete" })).unwrap();
    let r = app.run("insert", json!({ "name": "TAGGED", "at": [20, 0] })).unwrap();
    let i = r["handle"].as_str().unwrap().to_string();
    let mut h = Harness::builder().with_size(vec2(1600.0, 900.0)).build_ui_state(
        |ui, app: &mut CadApp| {
            app.logic(&ui.ctx().clone());
            app.ui(ui);
        },
        app,
    );
    let _ = h.state_mut().run("select", json!({ "handles": [i] }));
    let _ = h.state_mut().run("ui.istcad.properties", json!({ "on": true }));
    h.run_steps(4);
    let at = text_pos(&h, "PARTNO").expect("the attribute's tag is a row");
    click(&mut h, at + vec2(150.0, 6.0));
    h.run_steps(3);
    h.event(egui::Event::Text("B-2".into()));
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    let value = h.state().session.doc().unwrap().model.iter().find(|e| e.handle.hex() == i).and_then(|e| match &e.kind {
        cadcraft_engine::doc::EntityKind::Insert(ins) => ins.attribs.first().map(|a| a.text.value.clone()),
        _ => None,
    });
    assert_eq!(value.as_deref(), Some("B-2"));
}

/// A coordinate pair's pick button: the command line asks for a point, and the rows take it.
#[test]
fn istcad_properties_pick_a_point_for_coordinates() {
    let mut h = Harness::builder().with_size(vec2(1600.0, 900.0)).build_ui_state(
        |ui, app: &mut CadApp| {
            app.logic(&ui.ctx().clone());
            app.ui(ui);
        },
        istcad_app(),
    );
    h.run_steps(3);
    let circle = h.state().session.doc().ok().and_then(|d| {
        d.model.iter().find(|e| matches!(e.kind, cadcraft_engine::doc::EntityKind::Circle(_))).map(|e| e.handle.hex())
    });
    let Some(c) = circle else { panic!("the sample has a circle") };
    let _ = h.state_mut().run("select", json!({ "handles": [c] }));
    let _ = h.state_mut().run("ui.istcad.properties", json!({ "on": true }));
    h.run_steps(4);
    egui_kittest::kittest::Queryable::get_by_label(&h, "Pick point for Center X").click();
    h.run_steps(2);
    assert!(h.state().session.running.is_some(), "the command line asks for the point");
    h.state_mut().session.input(cadcraft_engine::Input::Point(cadcraft_geom::Vec2::new(123.0, 45.0))).unwrap();
    h.run_steps(3);
    let center = h.state().session.doc().unwrap().model.iter().find(|e| e.handle.hex() == c).and_then(|e| match &e.kind {
        cadcraft_engine::doc::EntityKind::Circle(k) => Some((k.center.x, k.center.y)),
        _ => None,
    });
    assert_eq!(center, Some((123.0, 45.0)));
}

/// Object snaps in the iStCAD window, as a user draws: LINE, the pointer a few pixels from a
/// circle's centre and then from a line's end; each click lands exactly on the snapped point.
#[test]
fn istcad_line_snaps_to_centre_and_end() {
    let mut h = Harness::builder().with_size(vec2(1600.0, 900.0)).build_ui_state(
        |ui, app: &mut CadApp| {
            app.logic(&ui.ctx().clone());
            app.ui(ui);
        },
        istcad_app(),
    );
    h.run_steps(3);
    let (centre, end) = {
        let d = h.state().session.doc().unwrap();
        let c = d.model.iter().find_map(|e| if let cadcraft_engine::doc::EntityKind::Circle(c) = &e.kind { Some(c.center.xy()) } else { None });
        let l = d.model.iter().find_map(|e| if let cadcraft_engine::doc::EntityKind::Line(l) = &e.kind { Some(l.b.xy()) } else { None });
        (c.expect("a circle"), l.expect("a line"))
    };
    h.state_mut().start("line");
    h.run_steps(2);
    let before = h.state().session.doc().unwrap().model.len();
    for target in [centre, end] {
        let xf = h.state().canvas.xf.expect("the canvas has drawn");
        let at = xf.to_screen(target) + vec2(3.0, -2.0);
        h.event(egui::Event::PointerMoved(at));
        h.run_steps(3);
        assert!(h.state().canvas.snap.is_some(), "a snap is found 3 px from {target:?}");
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE });
            h.run_steps(1);
        }
        h.run_steps(1);
    }
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    let d = h.state().session.doc().unwrap();
    assert_eq!(d.model.len(), before + 1, "one line drawn");
    let Some(cadcraft_engine::doc::EntityKind::Line(l)) = d.model.last().map(|e| &e.kind) else { panic!("the last object is the line") };
    assert!(l.a.xy().dist(centre) < 1e-9 && l.b.xy().dist(end) < 1e-9, "the line runs centre to end exactly: {l:?}");
}

/// Object snap tracking as in AutoCAD: rest on a line's end to acquire it, move away along its
/// horizontal, and the cursor follows the tracking path; a click lands on it.
#[test]
fn istcad_tracking_follows_an_acquired_point() {
    let mut h = Harness::builder().with_size(vec2(1600.0, 900.0)).with_step_dt(0.05).build_ui_state(
        |ui, app: &mut CadApp| {
            app.logic(&ui.ctx().clone());
            app.ui(ui);
        },
        istcad_app(),
    );
    h.run_steps(3);
    let end = h
        .state()
        .session
        .doc()
        .unwrap()
        .model
        .iter()
        .find_map(|e| if let cadcraft_engine::doc::EntityKind::Line(l) = &e.kind { Some(l.b.xy()) } else { None })
        .expect("a line");
    assert!(h.state().session.settings.otrack, "OTRACK is on at start");
    h.state_mut().start("line");
    h.run_steps(2);
    let xf = h.state().canvas.xf.expect("the canvas has drawn");
    // Rest on the end long enough to acquire it (each step is 50 ms).
    let on_end = xf.to_screen(end) + vec2(2.0, 1.0);
    for _ in 0..14 {
        h.event(egui::Event::PointerMoved(on_end));
        h.run_steps(1);
    }
    // Then away to the right, 1.5 px off its horizontal.
    let away = xf.to_screen(end) + vec2(90.0, 1.5);
    for _ in 0..3 {
        h.event(egui::Event::PointerMoved(away));
        h.run_steps(1);
    }
    assert!(h.state().canvas.track.is_some(), "a tracking path from the acquired end");
    let cursor = h.state().canvas.cursor.expect("a cursor");
    assert!((cursor.y - end.y).abs() < 1e-9, "the cursor sits on the end's horizontal: {cursor:?} vs {end:?}");
}

/// A layout tab is renamed in place: double-click, type, Enter. (Short steps, so the two clicks
/// fall within egui's double-click time.)
#[test]
fn istcad_layout_tab_renames_in_place() {
    let mut h = Harness::builder().with_size(vec2(1600.0, 900.0)).with_step_dt(0.02).build_ui_state(
        |ui, app: &mut CadApp| {
            app.logic(&ui.ctx().clone());
            app.ui(ui);
        },
        istcad_app(),
    );
    h.run_steps(3);
    let at = text_pos(&h, "Layout1").expect("the Layout1 tab is painted") + vec2(8.0, 6.0);
    click(&mut h, at);
    click(&mut h, at);
    h.run_steps(3);
    assert_eq!(h.state().istcad.renaming_layout(), Some("Layout1"), "a double-click opens the tab's name for editing");
    h.event(egui::Event::Text("Sheet A".into()));
    h.run_steps(1);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    let names: Vec<String> = h.state().session.doc().unwrap().layouts.iter().map(|l| l.name.clone()).collect();
    assert!(names.iter().any(|n| n == "Sheet A") && !names.iter().any(|n| n == "Layout1"), "{names:?}");
}
