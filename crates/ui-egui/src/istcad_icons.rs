//! iStCAD's monochrome line icons, drawn from the path data of iStCAD's own window
//! (`iStCAD_1/MainWindow.xaml` 1.0.233, iSt Solutions' work), plus icons in the same style (24-unit
//! grid, one stroke weight, CAD grips as small squares) for the commands iStCAD shows as text
//! glyphs on its edge toolbar.
//!
//! [`paint`] understands the subset of path syntax those icons use: absolute `M L H V A Z`, with
//! repeated coordinate pairs after `M`/`L`. Coordinates are in a square box of [`Glyph::box_size`]
//! units scaled to the target rectangle.

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2};

/// One iStCAD icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glyph {
    Undo,
    Redo,
    New,
    Open,
    Save,
    SaveAs,
    Select,
    Line,
    Circle,
    Arc,
    Polyline,
    Rectangle,
    Move,
    Trim,
    Dimension,
    Settings,
    Preferences,
    Menu,
    Copy,
    Erase,
    ZoomExtents,
    ZoomPrevious,
    Pan,
    Rotate,
    Plot,
    Layout,
    Regen,
}

impl Glyph {
    /// The side of the square the path coordinates are given in.
    pub fn box_size(self) -> f32 {
        match self {
            Glyph::Menu => 16.0,
            _ => 24.0,
        }
    }

    /// The icon's path data; `None` for shapes drawn directly ([`Glyph::Circle`], [`Glyph::Rectangle`]).
    fn path(self) -> Option<&'static str> {
        Some(match self {
            // iStCAD 1.0.233's title-bar icons.
            Glyph::Undo => "M9,5 L4,10 L9,15 M4,10 H15 A5,5 0 0 1 15,20 H10",
            Glyph::Redo => "M15,5 L20,10 L15,15 M20,10 H9 A5,5 0 0 0 9,20 H14",
            Glyph::New => "M6,3.5 H14 L19,8.5 V20.5 H6 Z M14,3.5 V8.5 H19 M12.5,12.5 V18 M9.75,15.25 H15.25",
            Glyph::Open => "M3,6.5 V5.5 H9 L11.5,8 H20.5 V19.5 H3 Z M3,11 H20.5",
            Glyph::Save => "M4,3.5 H17 L20,6.5 V20.5 H4 Z M7.5,3.5 V9 H16 V3.5 M7.5,20.5 V13.5 H16.5 V20.5",
            Glyph::SaveAs => "M4,3.5 H17 L20,6.5 V11 M4,3.5 V20.5 H10 M7.5,3.5 V9 H16 V3.5 M12,21 L12.6,18 L19,11.6 L21.4,14 L15,20.4 Z",
            Glyph::Select => "M5,3 L18,14 L12,15.5 L9.5,21 Z",
            // Drawing commands: the object with grips (small squares) at its defining points.
            Glyph::Line => "M7,17 L17,7 M4,17 H7 V20 H4 Z M17,4 H20 V7 H17 Z",
            Glyph::Arc => "M5,18 A9.5,9.5 0 0 1 19,8 M3.5,16.5 H6.5 V19.5 H3.5 Z M17.5,6.5 H20.5 V9.5 H17.5 Z",
            Glyph::Polyline => "M5,18 L9.5,8 L15,15 L19,6 M3.5,16.5 H6.5 V19.5 H3.5 Z M8,6.5 H11 V9.5 H8 Z M13.5,13.5 H16.5 V16.5 H13.5 Z M17.5,4.5 H20.5 V7.5 H17.5 Z",
            Glyph::Rectangle => "M5,6 H19 V18 H5 Z M3.5,4.5 H6.5 V7.5 H3.5 Z M17.5,16.5 H20.5 V19.5 H17.5 Z",
            Glyph::Move => "M12,3 V8.5 M12,15.5 V21 M3,12 H8.5 M15.5,12 H21 M9.5,5.5 L12,3 L14.5,5.5 M9.5,18.5 L12,21 L14.5,18.5 M5.5,9.5 L3,12 L5.5,14.5 M18.5,9.5 L21,12 L18.5,14.5 M9.5,9.5 H14.5 V14.5 H9.5 Z",
            Glyph::Pan => "M12,3 V9.5 M12,14.5 V21 M3,12 H9.5 M14.5,12 H21 M9.5,5.5 L12,3 L14.5,5.5 M9.5,18.5 L12,21 L14.5,18.5 M5.5,9.5 L3,12 L5.5,14.5 M18.5,9.5 L21,12 L18.5,14.5 M10.5,12 A1.5,1.5 0 1 0 13.5,12 A1.5,1.5 0 1 0 10.5,12",
            Glyph::Rotate => "M19,8 A8,8 0 1 0 20,13 M19,3.5 V8 H14.5 M11,12 A1,1 0 1 0 13,12 A1,1 0 1 0 11,12",
            Glyph::Trim => "M5,5 L19,19 M19,5 L5,19 M6,6 H11 V11 H6 Z M13,13 H18 V18 H13 Z",
            Glyph::Dimension => "M4,18 L18,4 M4,18 L4,13 M4,18 L9,18 M18,4 L13,4 M18,4 L18,9 M7,15 L15,7",
            Glyph::Settings => {
                "M12,3 V6 M12,18 V21 M3,12 H6 M18,12 H21 M5.6,5.6 L7.7,7.7 M16.3,16.3 L18.4,18.4 M18.4,5.6 L16.3,7.7 M7.7,16.3 L5.6,18.4 M12,8 A4,4 0 1 0 12.01,8"
            }
            Glyph::Preferences => "M4,5 H20 M4,12 H20 M4,19 H20 M8,3 V7 M15,10 V14 M11,17 V21",
            Glyph::Menu => "M2,3 H14 M2,7.5 H14 M2,12 H14",
            // In iStCAD's style, for commands iStCAD labels with text glyphs.
            Glyph::Copy => "M9,9 H20 V20 H9 Z M4,15 V4 H15 V6.5 M14.5,12 V17 M12,14.5 H17",
            Glyph::Erase => "M4,14.5 L12.5,6 L19,12.5 L11.5,20 H8 Z M8.25,10.25 L14.75,16.75 M13.5,20 H20",
            Glyph::ZoomExtents => {
                "M4,10.5 A6.5,6.5 0 1 1 17,10.5 A6.5,6.5 0 1 1 4,10.5 M15.2,15.2 L20.5,20.5 M7.5,9.5 V7.5 H9.5 M11.5,7.5 H13.5 V9.5 M13.5,11.5 V13.5 H11.5 M9.5,13.5 H7.5 V11.5"
            }
            Glyph::ZoomPrevious => "M4,10.5 A6.5,6.5 0 1 1 17,10.5 A6.5,6.5 0 1 1 4,10.5 M15.2,15.2 L20.5,20.5 M7.5,10.5 H13.5 M7.5,10.5 L9.75,8.25 M7.5,10.5 L9.75,12.75",
            Glyph::Plot => "M7,9 V3.5 H17 V9 M4,9 H20 V17 H4 Z M7,14 H17 V20.5 H7 Z",
            Glyph::Layout => "M3,4.5 H21 V19.5 H3 Z M3,9 H21 M9,9 V19.5",
            Glyph::Regen => "M19,12 A7,7 0 1 1 16.9,7 M17.5,3 V7.5 H13",
            Glyph::Circle => return None,
        })
    }

    /// Whether the shape is filled (the selection arrow).
    fn filled(self) -> bool {
        self == Glyph::Select
    }
}

/// Paint `glyph` into `rect` with `color`; `width` is the stroke width at a 24-point icon.
pub fn paint(p: &Painter, rect: Rect, glyph: Glyph, color: Color32, width: f32) {
    let side = rect.width().min(rect.height());
    let s = side / glyph.box_size();
    let origin = pos2(rect.center().x - side / 2.0, rect.center().y - side / 2.0);
    let stroke = Stroke::new(width * side / 24.0, color);
    match glyph {
        Glyph::Circle => {
            // The circle with its centre marked and a grip on its edge.
            let c = rect.center();
            p.circle_stroke(c, 8.0 * s, stroke);
            p.line_segment([c - egui::vec2(2.0 * s, 0.0), c + egui::vec2(2.0 * s, 0.0)], stroke);
            p.line_segment([c - egui::vec2(0.0, 2.0 * s), c + egui::vec2(0.0, 2.0 * s)], stroke);
            let grip = Rect::from_center_size(c + egui::vec2(8.0 * s, 0.0), egui::vec2(3.0 * s, 3.0 * s));
            p.rect_stroke(grip, 0.0, stroke, egui::StrokeKind::Middle);
            return;
        }
        _ => {}
    }
    let Some(data) = glyph.path() else { return };
    for (points, closed) in subpaths(data) {
        let pts: Vec<Pos2> = points.iter().map(|q| pos2(origin.x + q.x * s, origin.y + q.y * s)).collect();
        if pts.len() < 2 {
            continue;
        }
        if glyph.filled() && closed {
            p.add(Shape::convex_polygon(pts, color, stroke));
        } else if closed {
            p.add(Shape::closed_line(pts, stroke));
        } else {
            p.add(Shape::line(pts, stroke));
        }
    }
}

/// The path's subpaths as point lists in path units, arcs flattened, each with whether it closed.
pub fn subpaths(data: &str) -> Vec<(Vec<Pos2>, bool)> {
    let tokens = tokenize(data);
    let mut out = Vec::new();
    let mut current: Vec<Pos2> = Vec::new();
    let mut at = Pos2::ZERO;
    let mut start = Pos2::ZERO;
    let mut cmd = 'M';
    let mut i = 0;
    let num = |i: &mut usize| -> Option<f32> {
        match tokens.get(*i) {
            Some(Token::Num(v)) => {
                *i += 1;
                Some(*v)
            }
            _ => None,
        }
    };
    while i < tokens.len() {
        if let Some(Token::Cmd(c)) = tokens.get(i) {
            cmd = *c;
            i += 1;
            if cmd == 'Z' {
                if current.len() > 1 {
                    out.push((std::mem::take(&mut current), true));
                }
                at = start;
                continue;
            }
        }
        match cmd {
            'M' => {
                let (Some(x), Some(y)) = (num(&mut i), num(&mut i)) else { break };
                if current.len() > 1 {
                    out.push((std::mem::take(&mut current), false));
                }
                current.clear();
                at = pos2(x, y);
                start = at;
                current.push(at);
                // Further pairs after M are line-tos.
                cmd = 'L';
            }
            'L' => {
                let (Some(x), Some(y)) = (num(&mut i), num(&mut i)) else { break };
                at = pos2(x, y);
                current.push(at);
            }
            'H' => {
                let Some(x) = num(&mut i) else { break };
                at = pos2(x, at.y);
                current.push(at);
            }
            'V' => {
                let Some(y) = num(&mut i) else { break };
                at = pos2(at.x, y);
                current.push(at);
            }
            'A' => {
                let vals: Vec<f32> = (0..7).map_while(|_| num(&mut i)).collect();
                let [rx, ry, rot, large, sweep, x, y] = vals[..] else { break };
                let to = pos2(x, y);
                current.extend(arc_points(at, to, rx, ry, rot, large != 0.0, sweep != 0.0));
                at = to;
            }
            _ => {
                // An unsupported command: skip its token.
                i += 1;
            }
        }
    }
    if current.len() > 1 {
        out.push((current, false));
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Token {
    Cmd(char),
    Num(f32),
}

fn tokenize(data: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut number = String::new();
    let flush = |number: &mut String, out: &mut Vec<Token>| {
        if let Ok(v) = number.parse::<f32>() {
            out.push(Token::Num(v));
        }
        number.clear();
    };
    for c in data.chars() {
        match c {
            '0'..='9' | '.' => number.push(c),
            '-' => {
                flush(&mut number, &mut out);
                number.push(c);
            }
            c if c.is_ascii_alphabetic() => {
                flush(&mut number, &mut out);
                out.push(Token::Cmd(c.to_ascii_uppercase()));
            }
            _ => flush(&mut number, &mut out),
        }
    }
    flush(&mut number, &mut out);
    out
}

/// Points along an elliptical arc from `from` to `to` (SVG endpoint parameterisation), `to`
/// included and `from` left out.
fn arc_points(from: Pos2, to: Pos2, rx: f32, ry: f32, rotation_deg: f32, large: bool, sweep: bool) -> Vec<Pos2> {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx < 1e-6 || ry < 1e-6 || from == to {
        return vec![to];
    }
    let phi = rotation_deg.to_radians();
    let (sin, cos) = phi.sin_cos();
    let dx = (from.x - to.x) / 2.0;
    let dy = (from.y - to.y) / 2.0;
    let x1 = cos * dx + sin * dy;
    let y1 = -sin * dx + cos * dy;
    // Radii too small to reach are scaled up.
    let lambda = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
    if lambda > 1.0 {
        let k = lambda.sqrt();
        rx *= k;
        ry *= k;
    }
    let num = (rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1).max(0.0);
    let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let mut coef = if den > 0.0 { (num / den).sqrt() } else { 0.0 };
    if large == sweep {
        coef = -coef;
    }
    let cx1 = coef * rx * y1 / ry;
    let cy1 = -coef * ry * x1 / rx;
    let cx = cos * cx1 - sin * cy1 + (from.x + to.x) / 2.0;
    let cy = sin * cx1 + cos * cy1 + (from.y + to.y) / 2.0;
    let angle = |ux: f32, uy: f32| uy.atan2(ux);
    let theta1 = angle((x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut delta = angle((-x1 - cx1) / rx, (-y1 - cy1) / ry) - theta1;
    let tau = std::f32::consts::TAU;
    if sweep && delta < 0.0 {
        delta += tau;
    } else if !sweep && delta > 0.0 {
        delta -= tau;
    }
    let steps = ((delta.abs() / tau) * 48.0).ceil().max(4.0) as usize;
    (1..=steps)
        .map(|k| {
            let th = theta1 + delta * k as f32 / steps as f32;
            let (s, c) = th.sin_cos();
            pos2(cx + cos * rx * c - sin * ry * s, cy + sin * rx * c + cos * ry * s)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn istcad_icon_paths_split_into_their_strokes() {
        // Move: one M per stroke (four shafts, four heads, the box).
        assert_eq!(subpaths(Glyph::Move.path().unwrap_or_default()).len(), 9);
        // New: an outline that closes, then three open strokes.
        let new = subpaths(Glyph::New.path().unwrap_or_default());
        assert!(new.first().is_some_and(|(_, closed)| *closed));
        assert_eq!(new.len(), 4);
    }

    #[test]
    fn istcad_icon_arcs_end_where_the_path_says() {
        let arc = subpaths(Glyph::Arc.path().unwrap_or_default());
        let last = arc.first().and_then(|(pts, _)| pts.last().copied());
        assert!(last.is_some_and(|p| (p - pos2(19.0, 8.0)).length() < 1e-3), "{last:?}");
        // The near-full gear circle stays near its 4-unit radius around (12, 12).
        let gear = subpaths(Glyph::Settings.path().unwrap_or_default());
        let ring = gear.last().map(|(pts, _)| pts.clone()).unwrap_or_default();
        assert!(ring.len() > 10);
        assert!(ring.iter().all(|p| ((*p - pos2(12.0, 12.0)).length() - 4.0).abs() < 0.2));
    }

    #[test]
    fn istcad_icons_all_have_geometry() {
        for g in [
            Glyph::Undo,
            Glyph::Redo,
            Glyph::New,
            Glyph::Open,
            Glyph::Save,
            Glyph::SaveAs,
            Glyph::Select,
            Glyph::Line,
            Glyph::Arc,
            Glyph::Polyline,
            Glyph::Rectangle,
            Glyph::Move,
            Glyph::Pan,
            Glyph::Rotate,
            Glyph::ZoomPrevious,
            Glyph::Trim,
            Glyph::Dimension,
            Glyph::Settings,
            Glyph::Preferences,
            Glyph::Menu,
            Glyph::Copy,
            Glyph::Erase,
            Glyph::ZoomExtents,
            Glyph::Plot,
            Glyph::Layout,
            Glyph::Regen,
        ] {
            assert!(subpaths(g.path().unwrap_or_default()).iter().any(|(p, _)| p.len() >= 2), "{g:?}");
        }
    }
}
