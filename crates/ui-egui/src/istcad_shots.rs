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
