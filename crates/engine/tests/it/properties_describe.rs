//! `properties.describe`: the Properties palette's rows for text, mtext and dimensions, with
//! shared values, *VARIES*, units, the reason a dimension's text height can't be edited, and
//! the Geometry rows of lines, circles, polylines and block references.

use cadcraft_engine::Session;
use serde_json::{Value, json};

fn handle(r: &Value) -> String {
    r["handle"].as_str().or_else(|| r["handles"][0].as_str()).unwrap().to_string()
}

fn last(s: &Session) -> String {
    s.doc().unwrap().model.last().unwrap().handle.hex()
}

fn describe(s: &mut Session, p: Value) -> Value {
    s.execute("properties.describe", &p).unwrap()
}

/// The row `key` in group `group`.
fn row<'a>(desc: &'a Value, group: &str, key: &str) -> &'a Value {
    let g = desc["groups"].as_array().unwrap().iter().find(|g| g["name"] == group).unwrap_or_else(|| panic!("no group {group}: {desc}"));
    g["rows"].as_array().unwrap().iter().find(|r| r["key"] == key).unwrap_or_else(|| panic!("no row {key} in {group}: {g}"))
}

fn group_names(desc: &Value) -> Vec<String> {
    desc["groups"].as_array().unwrap().iter().map(|g| g["name"].as_str().unwrap().to_string()).collect()
}

#[test]
fn text_rows_share_values_and_vary() {
    let mut s = Session::new();
    let a = handle(&s.execute("text", &json!({ "at": [0, 0], "text": "A", "height": 2.5 })).unwrap());
    let b = handle(&s.execute("text", &json!({ "at": [0, 5], "text": "B", "height": 1, "justify": "MC" })).unwrap());
    let one = describe(&mut s, json!({ "handles": [a] }));
    assert_eq!(group_names(&one), ["General", "Text", "Geometry"]);
    let h = row(&one, "Text", "height");
    assert_eq!((h["value"].as_f64(), h["display"].as_str(), h["kind"].as_str()), (Some(2.5), Some("2.5000"), Some("distance")));
    assert_eq!(row(&one, "Text", "justify")["display"], "Left");
    assert_eq!(row(&one, "Text", "style")["value"], "Standard");
    assert!(row(&one, "Text", "style")["options"].as_array().unwrap().iter().any(|o| o["value"] == "Standard"));
    assert_eq!(row(&one, "Text", "text")["value"], "A");

    let both = describe(&mut s, json!({ "handles": [a, b] }));
    assert_eq!(both["count"], 2);
    let h = row(&both, "Text", "height");
    assert_eq!((h["varies"].as_bool(), h["display"].as_str(), h["value"].is_null()), (Some(true), Some("*VARIES*"), true));
    // Both are rotated 0 and in the same style.
    assert_eq!(row(&both, "Text", "rotation")["varies"], false);
    assert_eq!(row(&both, "Text", "style")["display"], "Standard");
    assert_eq!(row(&both, "Text", "justify")["varies"], true);
}

#[test]
fn mixed_selections_show_general_rows_unless_filtered() {
    let mut s = Session::new();
    let t = handle(&s.execute("text", &json!({ "at": [0, 0], "text": "A", "height": 1 })).unwrap());
    s.execute("circle", &json!({ "center": [0, 0], "radius": 1 })).unwrap();
    let c = last(&s);
    let mixed = describe(&mut s, json!({ "handles": [t, c] }));
    assert_eq!(group_names(&mixed), ["General"]);
    assert_eq!(mixed["types"], json!([{ "type": "Text", "count": 1 }, { "type": "Circle", "count": 1 }]));
    assert_eq!(row(&mixed, "General", "layer")["value"], "0");
    let texts = describe(&mut s, json!({ "handles": [t, c], "type": "text" }));
    assert_eq!((texts["filter"].as_str(), texts["count"].as_u64()), (Some("Text"), Some(1)));
    assert_eq!(group_names(&texts), ["General", "Text", "Geometry"]);
    // An unknown filter describes everything.
    assert_eq!(describe(&mut s, json!({ "handles": [t, c], "type": "nope" }))["count"], 2);
    // Nothing selected, or handles that name nothing: no rows.
    assert_eq!(describe(&mut s, json!({ "handles": [] }))["groups"], json!([]));
    assert_eq!(describe(&mut s, json!({ "handles": ["FFFFFF"] }))["count"], 0);
}

#[test]
fn mtext_rows() {
    let mut s = Session::new();
    let r = s.execute("mtext", &json!({ "at": [0, 0], "text": "one\\Ptwo", "height": 3, "width": 40, "justify": "MC", "lineSpacing": 1.5 })).unwrap();
    let m = handle(&r);
    let d = describe(&mut s, json!({ "handles": [m] }));
    assert_eq!(group_names(&d), ["General", "Text", "Geometry"]);
    assert_eq!(row(&d, "Text", "justify")["value"], "MC");
    assert_eq!(row(&d, "Text", "justify")["display"], "Middle center");
    assert_eq!(row(&d, "Text", "text")["kind"], "contents");
    assert_eq!(row(&d, "Text", "lineSpacing")["value"], 1.5);
    // Line space distance: factor × height × 5/3.
    let dist = row(&d, "Text", "lineSpacingDistance")["value"].as_f64().unwrap();
    assert!((dist - 1.5 * 3.0 * 5.0 / 3.0).abs() < 1e-9, "{dist}");
    assert_eq!(row(&d, "Text", "lineSpacingStyle")["value"], "atLeast");
    assert_eq!(row(&d, "Text", "width")["value"], 40.0);
    let cols = row(&d, "Text", "columns");
    assert_eq!((cols["display"].as_str(), cols["editable"].as_bool()), (Some("None"), Some(false)));
}

#[test]
fn dimension_rows_come_from_the_style_and_its_overrides() {
    let mut s = Session::new();
    s.execute("dimlinear", &json!({ "p1": [0, 0], "p2": [10, 0], "at": [5, 3] })).unwrap();
    let dm = last(&s);
    let d = describe(&mut s, json!({ "handles": [dm] }));
    assert_eq!(group_names(&d), ["General", "Misc", "Lines & Arrows", "Text", "Fit", "Primary Units", "Alternate Units", "Tolerances"]);
    let th = row(&d, "Text", "textHeight");
    assert_eq!((th["value"].as_f64(), th["editable"].as_bool(), th.get("overridden")), (Some(0.18), Some(true), None));
    assert_eq!(row(&d, "Misc", "dimStyle")["value"], "Standard");
    assert_eq!(row(&d, "Text", "textAbove")["display"], "Centered");
    assert_eq!(row(&d, "Primary Units", "decimals")["display"], "0.0000");
    assert_eq!(row(&d, "Lines & Arrows", "arrowBlock1")["display"], "Closed filled");
    assert_eq!(row(&d, "Lines & Arrows", "dimLineColor")["value"], "ByBlock");
    let m = row(&d, "Text", "measurement");
    assert_eq!((m["editable"].as_bool(), m["display"].as_str()), (Some(false), Some("10.0000")));
    assert_eq!(row(&d, "Text", "textOverride")["value"], "");

    // A Properties edit is an override of this dimension; the row says so.
    let r = s.execute("properties.set", &json!({ "handles": [dm], "textHeight": 0.3 })).unwrap();
    assert_eq!(r["ignored"], json!([]));
    let d = describe(&mut s, json!({ "handles": [dm] }));
    let th = row(&d, "Text", "textHeight");
    assert_eq!((th["value"].as_f64(), th["overridden"].as_bool()), (Some(0.3), Some(true)));
    // The style itself is unchanged.
    let st = s.execute("dimstyle.list", &json!({ "name": "Standard" })).unwrap();
    assert_eq!(st["variables"]["DIMTXT"], 0.18);
}

#[test]
fn a_fixed_height_text_style_decides_dimension_text_height() {
    let mut s = Session::new();
    s.execute("style", &json!({ "name": "Fixed", "height": 0.5, "current": false })).unwrap();
    s.execute("dimstyle", &json!({ "name": "FixedDims", "textStyle": "Fixed" })).unwrap();
    s.execute("dimlinear", &json!({ "p1": [0, 0], "p2": [10, 0], "at": [5, 3] })).unwrap();
    let dm = last(&s);
    let d = describe(&mut s, json!({ "handles": [dm] }));
    let th = row(&d, "Text", "textHeight");
    assert_eq!((th["editable"].as_bool(), th["display"].as_str()), (Some(false), Some("0.5000")));
    assert!(th["note"].as_str().unwrap().contains("Fixed"), "{th}");
    // Fixed heights that differ vary, whatever their DIMTXT says.
    s.execute("style", &json!({ "name": "Large", "height": 0.75, "current": false })).unwrap();
    s.execute("dimstyle", &json!({ "name": "LargeDims", "textStyle": "Large" })).unwrap();
    s.execute("dimlinear", &json!({ "p1": [0, 10], "p2": [10, 10], "at": [5, 13] })).unwrap();
    let large = last(&s);
    let d = describe(&mut s, json!({ "handles": [dm, large] }));
    let th = row(&d, "Text", "textHeight");
    assert_eq!((th["editable"].as_bool(), th["display"].as_str()), (Some(false), Some("*VARIES*")));
}

#[test]
fn measurements_read_as_each_dimension_writes_them() {
    let mut s = Session::new();
    let shown =
        |s: &mut Session, hs: Value| row(&describe(s, json!({ "handles": hs })), "Text", "measurement")["display"].as_str().unwrap().to_string();
    s.execute("dimangular", &json!({ "vertex": [0, 0], "p1": [10, 0], "p2": [0, 10], "at": [3, 3] })).unwrap();
    let angle = last(&s);
    assert_eq!(shown(&mut s, json!([angle])), "90°");
    // In the dimension's angle format.
    s.execute("properties.set", &json!({ "handles": [angle], "angularUnit": 3, "angularDecimals": 2 })).unwrap();
    assert_eq!(shown(&mut s, json!([angle])), "1.57r");
    // The same length written at two precisions varies.
    s.execute("dimlinear", &json!({ "p1": [0, 0], "p2": [10, 0], "at": [5, 3] })).unwrap();
    let a = last(&s);
    s.execute("dimlinear", &json!({ "p1": [0, 20], "p2": [10, 20], "at": [5, 23] })).unwrap();
    let b = last(&s);
    assert_eq!(shown(&mut s, json!([a, b])), "10.0000");
    s.execute("properties.set", &json!({ "handles": [b], "precision": 1 })).unwrap();
    assert_eq!(shown(&mut s, json!([a, b])), "*VARIES*");
}

#[test]
fn rows_show_lengths_in_the_drawing_units() {
    let mut s = Session::new();
    s.execute("setvar", &json!({ "name": "LUNITS", "value": 4 })).unwrap();
    let t = handle(&s.execute("text", &json!({ "at": [0, 0], "text": "A", "height": 18 })).unwrap());
    let d = describe(&mut s, json!({ "handles": [t] }));
    assert_eq!(row(&d, "Text", "height")["display"], "1'-6\"");
    assert_eq!(row(&d, "Text", "height")["value"], 18.0);
}

#[test]
fn hostile_requests_never_panic() {
    let mut s = Session::new();
    let t = handle(&s.execute("text", &json!({ "at": [0, 0], "text": "A", "height": 1 })).unwrap());
    for p in [json!({}), json!({ "handles": "x" }), json!({ "handles": [t, t, t] }), json!({ "handles": [t], "type": 5 }), json!({ "type": "Text" })]
    {
        let _ = s.execute("properties.describe", &p);
    }
    // Duplicates count once.
    assert_eq!(describe(&mut s, json!({ "handles": [t, t] }))["count"], 1);
}

#[test]
fn rotations_show_as_the_drawing_measures_directions() {
    let mut s = Session::new();
    s.execute("text", &json!({ "at": [0, 0], "text": "N", "height": 1, "rotation": 2.22729505553431 })).unwrap();
    // Clockwise from north (ANGBASE 90, ANGDIR 1): AutoCAD shows the text turned 2.23° as 87.77
    // (#1275); the value stays from +X counterclockwise, as properties.set takes it.
    for (name, value) in [("ANGDIR", json!(1)), ("ANGBASE", json!(90)), ("AUPREC", json!(2))] {
        s.execute("setvar", &json!({ "name": name, "value": value })).unwrap();
    }
    let h = last(&s);
    let desc = describe(&mut s, json!({ "handles": [h] }));
    let r = row(&desc, "Text", "rotation");
    assert_eq!(r["kind"], "direction");
    assert_eq!(r["display"], "87.77");
    assert!((r["value"].as_f64().unwrap() - 2.22729505553431).abs() < 1e-9);
}

#[test]
fn empty_arrowheads_read_as_the_ones_drawn() {
    use cadcraft_engine::doc::DimStyle;
    use cadcraft_engine::properties::{DIM_PROPS, dim_value};
    let st = serde_json::to_value(DimStyle { arrow_block: "_DOT".into(), ..DimStyle::default() }).unwrap();
    let p = |field: &str| DIM_PROPS.iter().find(|p| p.field == field).unwrap();
    // An empty per-end arrowhead draws DIMBLK's; an empty leader arrowhead is closed filled.
    assert_eq!(dim_value(p("arrowBlock1"), &st), json!("_DOT"));
    assert_eq!(dim_value(p("arrowBlock"), &st), json!("_DOT"));
    assert_eq!(dim_value(p("leaderArrowBlock"), &st), json!(""));
}

/// Float noise is no difference between two objects' values (7 and 7.0000000000001), but values
/// the display rounds alike still vary (7 and 7.000001 both show as 7.0000).
#[test]
fn float_noise_does_not_vary_but_a_real_difference_does() {
    let mut s = Session::new();
    let a = handle(&s.execute("text", &json!({ "at": [0, 0], "text": "A", "height": 7.0 })).unwrap());
    let b = handle(&s.execute("text", &json!({ "at": [0, 5], "text": "B", "height": 7.0 + 1e-13 })).unwrap());
    let both = describe(&mut s, json!({ "handles": [a.clone(), b.clone()] }));
    let h = row(&both, "Text", "height");
    assert_eq!((h["varies"].as_bool(), h["display"].as_str()), (Some(false), Some("7.0000")));
    s.execute("properties.set", &json!({ "handles": [b.clone()], "height": 7.000001 })).unwrap();
    let both = describe(&mut s, json!({ "handles": [a, b] }));
    assert_eq!(row(&both, "Text", "height")["varies"], true);
}

/// The row `key` in group `group`, as a number.
fn num(desc: &Value, group: &str, key: &str) -> f64 {
    row(desc, group, key)["value"].as_f64().unwrap_or_else(|| panic!("{key} is not a number: {desc}"))
}

#[test]
fn lines_have_geometry_rows_that_edit_every_line_one_axis_at_a_time() {
    let mut s = Session::new();
    s.execute("line", &json!({ "points": [[0, 0], [10, 0]] })).unwrap();
    let a = last(&s);
    s.execute("line", &json!({ "points": [[0, 5], [10, 8]] })).unwrap();
    let b = last(&s);
    let both = describe(&mut s, json!({ "handles": [a, b] }));
    assert_eq!(group_names(&both), ["General", "Geometry"]);
    assert_eq!(row(&both, "Geometry", "startX")["varies"], false);
    assert_eq!(row(&both, "Geometry", "startY")["varies"], true);
    assert!(num(&describe(&mut s, json!({ "handles": [b] })), "Geometry", "length") > 10.0);
    // Start X moves both starts across and keeps each one's Y.
    s.execute("properties.set", &json!({ "handles": [a, b], "startX": -4 })).unwrap();
    let after = describe(&mut s, json!({ "handles": [b] }));
    assert_eq!((num(&after, "Geometry", "startX"), num(&after, "Geometry", "startY")), (-4.0, 5.0));
    let after = describe(&mut s, json!({ "handles": [a] }));
    assert_eq!(num(&after, "Geometry", "startY"), 0.0);
}

#[test]
fn shapes_of_one_type_have_their_own_rows() {
    let mut s = Session::new();
    s.execute("circle", &json!({ "center": [3, 4], "radius": 2 })).unwrap();
    let c = last(&s);
    let circle = describe(&mut s, json!({ "handles": [c] }));
    assert!(num(&circle, "Geometry", "area") > num(&circle, "Geometry", "radius"));
    s.execute("properties.set", &json!({ "handles": [c], "centerY": 9 })).unwrap();
    let moved = describe(&mut s, json!({ "handles": [c] }));
    assert_eq!((num(&moved, "Geometry", "centerX"), num(&moved, "Geometry", "centerY")), (3.0, 9.0));

    s.execute("pline", &json!({ "vertices": [[0, 0], [4, 0], [4, 3]], "closed": false })).unwrap();
    let pl = last(&s);
    let open = describe(&mut s, json!({ "handles": [pl] }));
    assert_eq!(row(&open, "Misc", "closed")["value"], false);
    s.execute("properties.set", &json!({ "handles": [pl], "closed": true })).unwrap();
    let closed = describe(&mut s, json!({ "handles": [pl] }));
    assert!(num(&closed, "Geometry", "length") > num(&open, "Geometry", "length"));
}

#[test]
fn a_block_reference_moves_with_its_attributes() {
    let mut s = Session::new();
    s.execute("attdef", &json!({ "tag": "T", "default": "one", "at": [0, 1] })).unwrap();
    s.execute("line", &json!({ "points": [[0, 0], [10, 0]] })).unwrap();
    let hs: Vec<String> = s.doc().unwrap().model.iter().map(|e| e.handle.hex()).collect();
    s.execute("block", &json!({ "name": "B", "base": [0, 0], "handles": hs, "keep": "delete" })).unwrap();
    s.execute("insert", &json!({ "name": "B", "at": [100, 0] })).unwrap();
    let i = last(&s);
    let attrib_x = |s: &Session| {
        let d = s.doc().unwrap();
        match &d.model.last().unwrap().kind {
            cadcraft_engine::doc::EntityKind::Insert(ins) => ins.attribs.first().map(|a| a.text.insert.x),
            _ => None,
        }
    };
    let before = attrib_x(&s);
    assert_eq!(row(&describe(&mut s, json!({ "handles": [i] })), "Misc", "name")["value"], "B");
    s.execute("properties.set", &json!({ "handles": [i], "positionX": 130 })).unwrap();
    let after = describe(&mut s, json!({ "handles": [i] }));
    assert_eq!(num(&after, "Geometry", "positionX"), 130.0);
    if let (Some(b), Some(a)) = (before, attrib_x(&s)) {
        assert!((a - b - 30.0).abs() < 1e-9, "the attribute moved with the reference");
    }
}

#[test]
fn ellipses_polyline_vertices_and_block_scales_are_editable() {
    let mut s = Session::new();
    s.execute("ellipse", &json!({ "center": [0, 0], "major": [10, 0], "ratio": 0.5 })).unwrap();
    let el = last(&s);
    let rows = describe(&mut s, json!({ "handles": [el] }));
    assert_eq!(row(&rows, "Geometry", "majorRadius")["editable"], true);
    s.execute("properties.set", &json!({ "handles": [el], "minorRadius": 2, "centerZ": 3 })).unwrap();
    let rows = describe(&mut s, json!({ "handles": [el] }));
    assert_eq!((num(&rows, "Geometry", "minorRadius"), num(&rows, "Geometry", "centerZ")), (2.0, 3.0));
    // The major radius grows and the minor one stays.
    s.execute("properties.set", &json!({ "handles": [el], "majorRadius": 20 })).unwrap();
    let rows = describe(&mut s, json!({ "handles": [el] }));
    assert!((num(&rows, "Geometry", "minorRadius") - 2.0).abs() < 1e-9 && num(&rows, "Geometry", "majorRadius") > 10.0);

    s.execute("pline", &json!({ "vertices": [[0, 0], [4, 0], [4, 3]], "closed": false })).unwrap();
    let pl = last(&s);
    let before = describe(&mut s, json!({ "handles": [pl] }));
    let r = s.execute("properties.set", &json!({ "handles": [pl], "vertex": 3, "vertexY": 9, "elevation": 1 })).unwrap();
    assert_eq!(r["ignored"], json!([]), "{r}");
    let after = describe(&mut s, json!({ "handles": [pl] }));
    assert!(num(&after, "Geometry", "length") > num(&before, "Geometry", "length"));
    assert_eq!(num(&after, "Geometry", "elevation"), 1.0);
    // A vertex the polyline doesn't have changes nothing.
    let r = s.execute("properties.set", &json!({ "handles": [pl], "vertex": 9, "vertexX": 1 })).unwrap();
    assert!(r["ignored"].as_array().unwrap().iter().any(|k| k == "vertexX"), "{r}");

    s.execute("line", &json!({ "points": [[0, 0], [1, 0]] })).unwrap();
    let hs = vec![last(&s)];
    s.execute("block", &json!({ "name": "B", "base": [0, 0], "handles": hs, "keep": "delete" })).unwrap();
    s.execute("insert", &json!({ "name": "B", "at": [5, 5] })).unwrap();
    let i = last(&s);
    s.execute("properties.set", &json!({ "handles": [i], "scaleX": 2 })).unwrap();
    let rows = describe(&mut s, json!({ "handles": [i] }));
    assert_eq!((num(&rows, "Geometry", "scaleX"), num(&rows, "Geometry", "scaleY")), (2.0, 1.0));
}

#[test]
fn leaders_and_multileaders_are_described_and_edited() {
    let mut s = Session::new();
    s.execute("leader", &json!({ "points": [[0, 0], [5, 5], [10, 5]] })).unwrap();
    let l = last(&s);
    let rows = describe(&mut s, json!({ "handles": [l] }));
    assert_eq!(row(&rows, "Lines & Arrows", "arrow")["value"], true);
    assert_eq!(row(&rows, "Misc", "dimStyle")["value"], "Standard");
    let r = s.execute("properties.set", &json!({ "handles": [l], "arrow": false, "leaderType": "spline", "vertex": 2, "vertexY": 8 })).unwrap();
    assert_eq!(r["ignored"], json!([]), "{r}");
    let rows = describe(&mut s, json!({ "handles": [l] }));
    assert_eq!((row(&rows, "Lines & Arrows", "arrow")["display"].as_str(), row(&rows, "Lines & Arrows", "leaderType")["value"].as_str()), (Some("None"), Some("spline")));

    s.execute("mleader", &json!({ "points": [[0, 0], [10, 10]], "text": "NOTE" })).unwrap();
    let m = last(&s);
    let rows = describe(&mut s, json!({ "handles": [m] }));
    for (group, key) in [("Leaders", "arrowheadSize"), ("Text", "text"), ("Geometry", "landingX"), ("Misc", "mleaderStyle")] {
        assert_eq!(row(&rows, group, key)["editable"], true, "{group} {key}");
    }
    let x0 = num(&rows, "Geometry", "landingX");
    let r = s
        .execute("properties.set", &json!({ "handles": [m], "landingX": x0 + 5.0, "text": "NEW", "horizontalLanding": false, "arrowheadSize": 3 }))
        .unwrap();
    assert_eq!(r["ignored"], json!([]), "{r}");
    let rows = describe(&mut s, json!({ "handles": [m] }));
    assert_eq!(num(&rows, "Geometry", "landingX"), x0 + 5.0);
    assert_eq!(row(&rows, "Text", "text")["value"], "NEW");
    assert_eq!(row(&rows, "Leaders", "horizontalLanding")["value"], false);
    assert_eq!(num(&rows, "Leaders", "arrowheadSize"), 3.0);
    assert!(s.execute("properties.set", &json!({ "handles": [m], "mleaderStyle": "nope" })).is_err());
}

#[test]
fn a_block_reference_can_show_another_block() {
    let mut s = Session::new();
    for name in ["A", "B"] {
        s.execute("line", &json!({ "points": [[0, 0], [1, 0]] })).unwrap();
        let hs = vec![last(&s)];
        s.execute("block", &json!({ "name": name, "base": [0, 0], "handles": hs, "keep": "delete" })).unwrap();
    }
    s.execute("insert", &json!({ "name": "A", "at": [5, 5] })).unwrap();
    let i = last(&s);
    let rows = describe(&mut s, json!({ "handles": [i] }));
    assert!(row(&rows, "Misc", "name")["options"].as_array().unwrap().iter().any(|o| o["value"] == "B"));
    s.execute("properties.set", &json!({ "handles": [i], "name": "b" })).unwrap();
    assert_eq!(row(&describe(&mut s, json!({ "handles": [i] })), "Misc", "name")["value"], "B");
    assert!(s.execute("properties.set", &json!({ "handles": [i], "name": "missing" })).is_err());
}

#[test]
fn a_multileader_s_block_content_is_edited() {
    let mut s = Session::new();
    for name in ["A", "B"] {
        s.execute("circle", &json!({ "center": [0, 0], "radius": 1 })).unwrap();
        let hs = vec![last(&s)];
        s.execute("block", &json!({ "name": name, "base": [0, 0], "handles": hs, "keep": "delete" })).unwrap();
    }
    s.execute("mleader", &json!({ "points": [[0, 0], [10, 10]], "block": "A" })).unwrap();
    let m = last(&s);
    let rows = describe(&mut s, json!({ "handles": [m] }));
    assert_eq!(row(&rows, "Block", "sourceBlock")["value"], "A");
    let r = s.execute("properties.set", &json!({ "handles": [m], "sourceBlock": "B", "blockScale": 2, "blockRotation": 30 })).unwrap();
    assert_eq!(r["ignored"], json!([]), "{r}");
    let rows = describe(&mut s, json!({ "handles": [m] }));
    assert_eq!((row(&rows, "Block", "sourceBlock")["value"].as_str(), num(&rows, "Block", "blockScale")), (Some("B"), 2.0));
    assert!((num(&rows, "Block", "blockRotation") - 30.0).abs() < 1e-9);
}

#[test]
fn splines_rays_and_hatches_have_their_rows() {
    let mut s = Session::new();
    s.execute("spline", &json!({ "fit": [[0, 0], [5, 5], [10, 0]] })).unwrap();
    let sp = last(&s);
    let before = describe(&mut s, json!({ "handles": [sp] }));
    assert_eq!(row(&before, "Geometry", "closed")["editable"], true);
    let r = s.execute("properties.set", &json!({ "handles": [sp], "vertex": 2, "vertexY": 20 })).unwrap();
    assert_eq!(r["ignored"], json!([]), "{r}");
    let after = describe(&mut s, json!({ "handles": [sp] }));
    assert!(num(&after, "Geometry", "length") > num(&before, "Geometry", "length"));

    s.execute("ray", &json!({ "base": [0, 0], "through": [1, 0] })).unwrap();
    let ray = last(&s);
    s.execute("properties.set", &json!({ "handles": [ray], "secondPointX": 0, "secondPointY": 3 })).unwrap();
    let rows = describe(&mut s, json!({ "handles": [ray] }));
    assert!((num(&rows, "Geometry", "directionY") - 1.0).abs() < 1e-9, "the ray turns to the second point: {rows}");

    // Away from the spline and the ray, so the boundary is the rectangle alone.
    s.execute("rectang", &json!({ "p1": [100, -50], "p2": [110, -40] })).unwrap();
    let r = s.execute("hatch", &json!({ "points": [[105, -45]], "pattern": "ANSI31" })).unwrap();
    let h = handle(&r);
    let rows = describe(&mut s, json!({ "handles": [h] }));
    assert!((num(&rows, "Geometry", "area") - 100.0).abs() < 1e-6, "{rows}");
    let r = s.execute("properties.set", &json!({ "handles": [h], "originX": 2, "islandStyle": 2, "background": 3, "associative": false })).unwrap();
    assert_eq!(r["ignored"], json!([]), "{r}");
    let rows = describe(&mut s, json!({ "handles": [h] }));
    assert_eq!((num(&rows, "Pattern", "originX"), row(&rows, "Pattern", "islandStyle")["display"].as_str()), (2.0, Some("Ignore")));
    assert_ne!(row(&rows, "Pattern", "background")["value"], "None");
}

#[test]
fn tables_helixes_3d_polylines_and_attribute_definitions_are_edited() {
    let mut s = Session::new();
    s.execute("table", &json!({ "at": [0, 20], "cells": [["1", ""], ["2", ""]], "rowHeight": 1.0, "colWidth": 3.0 })).unwrap();
    let t = last(&s);
    let rows = describe(&mut s, json!({ "handles": [t] }));
    let width = num(&rows, "Table", "tableWidth");
    let r = s.execute("properties.set", &json!({ "handles": [t], "tableWidth": width * 2.0, "positionX": 5 })).unwrap();
    assert_eq!(r["ignored"], json!([]), "{r}");
    let rows = describe(&mut s, json!({ "handles": [t] }));
    assert!((num(&rows, "Table", "tableWidth") - width * 2.0).abs() < 1e-9);
    assert_eq!(num(&rows, "Geometry", "positionX"), 5.0);

    s.execute("helix", &json!({ "center": [0, 0], "baseRadius": 2, "height": 10, "turns": 5 })).unwrap();
    let hx = last(&s);
    s.execute("properties.set", &json!({ "handles": [hx], "turns": 10, "topRadius": 1 })).unwrap();
    let rows = describe(&mut s, json!({ "handles": [hx] }));
    assert!(num(&rows, "Geometry", "turnHeight") < 2.0, "more turns, closer together: {rows}");

    s.execute("3dpoly", &json!({ "points": [[0, 0, 0], [3, 0, 0], [3, 4, 0]] })).unwrap();
    let p3 = last(&s);
    let before = num(&describe(&mut s, json!({ "handles": [p3] })), "Geometry", "length");
    s.execute("properties.set", &json!({ "handles": [p3], "vertex": 3, "vertexZ": 12 })).unwrap();
    assert!(num(&describe(&mut s, json!({ "handles": [p3] })), "Geometry", "length") > before);

    s.execute("attdef", &json!({ "tag": "T", "default": "one", "at": [0, 1] })).unwrap();
    let ad = last(&s);
    let rows = describe(&mut s, json!({ "handles": [ad] }));
    assert_eq!(row(&rows, "Attribute", "tag")["value"], "T");
    let r = s.execute("properties.set", &json!({ "handles": [ad], "tag": "part", "prompt": "Part?", "text": "two", "invisible": true })).unwrap();
    assert_eq!(r["ignored"], json!([]), "{r}");
    let rows = describe(&mut s, json!({ "handles": [ad] }));
    assert_eq!((row(&rows, "Attribute", "tag")["value"].as_str(), row(&rows, "Attribute", "text")["value"].as_str()), (Some("PART"), Some("two")));
    assert_eq!(row(&rows, "Misc", "invisible")["value"], true);
}

#[test]
fn a_multileader_s_leader_lines_come_and_go() {
    let mut s = Session::new();
    s.execute("mleader", &json!({ "points": [[0, 0], [10, 10]], "text": "A" })).unwrap();
    let m = last(&s);
    s.execute("properties.set", &json!({ "handles": [m], "leaderType": "none" })).unwrap();
    assert_eq!(row(&describe(&mut s, json!({ "handles": [m] })), "Leaders", "leaderType")["display"], "None");
    s.execute("properties.set", &json!({ "handles": [m], "leaderType": "straight" })).unwrap();
    assert_eq!(row(&describe(&mut s, json!({ "handles": [m] })), "Leaders", "leaderType")["display"], "Straight");
}
