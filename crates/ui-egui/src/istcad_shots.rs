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
