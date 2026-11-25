//! Lasso selection: pressing on empty space and dragging draws a free-form selection area, as in
//! AutoCAD. Dragging to the right makes a window lasso (objects entirely inside), to the left a
//! crossing lasso (objects inside or crossed); Space while dragging cycles Window → Crossing →
//! Fence (objects the path crosses). A click (no drag) still starts the two-click rectangular
//! window. The selecting itself is the engine's ([`cadcraft_engine::Session::lasso_select`]).

use cadcraft_engine::select::LassoKind;
use egui::{Color32, Mesh, Painter, Pos2, Shape, Stroke};

/// Points closer than this (screen points) to the last one are not kept.
const MIN_STEP: f32 = 2.0;
/// Most points a lasso keeps (a long drag stops growing the path).
const MAX_POINTS: usize = 4096;
/// How far the pointer must move sideways before the drag's direction decides the kind.
const DECIDE: f32 = 3.0;

/// A lasso being dragged, in screen points.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Lasso {
    pub(crate) points: Vec<Pos2>,
    /// The kind Space picked; `None`: decided by the drag's direction.
    chosen: Option<LassoKind>,
}

impl Lasso {
    pub(crate) fn new(at: Pos2) -> Self {
        Lasso { points: vec![at], chosen: None }
    }

    /// The pointer is at `p`.
    pub(crate) fn add(&mut self, p: Pos2) {
        if self.points.len() >= MAX_POINTS || !p.x.is_finite() || !p.y.is_finite() {
            return;
        }
        if self.points.last().is_none_or(|last| last.distance(p) >= MIN_STEP) {
            self.points.push(p);
        }
    }

    /// Whether the drag has gone far enough to be drawn as a lasso.
    pub(crate) fn is_drawn(&self) -> bool {
        self.points.len() >= 3
    }

    /// The kind: Space's choice, else right = window, left = crossing (the first sideways move).
    pub(crate) fn kind(&self) -> LassoKind {
        if let Some(k) = self.chosen {
            return k;
        }
        let first = self.points.first().copied().unwrap_or(Pos2::ZERO);
        match self.points.iter().find(|p| (p.x - first.x).abs() > DECIDE) {
            Some(p) if p.x < first.x => LassoKind::Crossing,
            _ => LassoKind::Window,
        }
    }

    /// Space: the next kind.
    pub(crate) fn cycle(&mut self) {
        self.chosen = Some(self.kind().next());
    }
}

/// Draw `lasso` with the area colours of a window or crossing selection: window a solid outline,
/// crossing a dashed one, fence a dashed path without fill.
pub(crate) fn paint(p: &Painter, lasso: &Lasso, window: (Option<Color32>, Stroke), crossing: (Option<Color32>, Stroke)) {
    let pts = &lasso.points;
    match lasso.kind() {
        LassoKind::Fence => {
            p.extend(Shape::dashed_line(pts, crossing.1, 5.0, 3.0));
        }
        kind => {
            let (fill, stroke) = if kind == LassoKind::Crossing { crossing } else { window };
            if let Some(fill) = fill {
                p.add(Shape::mesh(fill_mesh(pts, fill)));
            }
            let mut closed = pts.clone();
            if let Some(first) = pts.first() {
                closed.push(*first);
            }
            if kind == LassoKind::Crossing {
                p.extend(Shape::dashed_line(&closed, stroke, 5.0, 3.0));
            } else {
                p.add(Shape::line(closed, stroke));
            }
        }
    }
}

/// A filled mesh of the (possibly concave) polygon `pts`, by ear clipping on a thinned copy.
fn fill_mesh(pts: &[Pos2], fill: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    let step = (pts.len() / 300).max(1);
    let poly: Vec<Pos2> = pts.iter().step_by(step).copied().collect();
    for [a, b, c] in triangulate(&poly) {
        let base = mesh.vertices.len() as u32;
        for i in [a, b, c] {
            mesh.colored_vertex(poly[i], fill);
        }
        mesh.add_triangle(base, base + 1, base + 2);
    }
    mesh
}

/// Twice the signed area of `pts` (positive when counter-clockwise in a y-up frame).
fn signed_area2(pts: &[Pos2]) -> f32 {
    let n = pts.len();
    (0..n).map(|i| pts[i].x * pts[(i + 1) % n].y - pts[(i + 1) % n].x * pts[i].y).sum()
}

/// Triangles covering the simple polygon `pts` (ear clipping). A self-crossing path still gets
/// triangles; where it crosses itself the fill is approximate, as the lasso only shows the area.
pub(crate) fn triangulate(pts: &[Pos2]) -> Vec<[usize; 3]> {
    let n = pts.len();
    if n < 3 {
        return Vec::new();
    }
    let ccw = signed_area2(pts) > 0.0;
    let mut idx: Vec<usize> = (0..n).collect();
    let mut out = Vec::with_capacity(n - 2);
    let cross = |a: Pos2, b: Pos2, c: Pos2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let convex = |a: Pos2, b: Pos2, c: Pos2| if ccw { cross(a, b, c) > 0.0 } else { cross(a, b, c) < 0.0 };
    let inside = |p: Pos2, a: Pos2, b: Pos2, c: Pos2| {
        let (d1, d2, d3) = (cross(a, b, p), cross(b, c, p), cross(c, a, p));
        let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
        let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
        !(neg && pos)
    };
    let mut guard = 0usize;
    while idx.len() > 3 && guard < n * n {
        guard += 1;
        let m = idx.len();
        let mut clipped = false;
        for i in 0..m {
            let (ia, ib, ic) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            let (a, b, c) = (pts[ia], pts[ib], pts[ic]);
            if !convex(a, b, c) {
                continue;
            }
            if idx.iter().any(|&j| j != ia && j != ib && j != ic && inside(pts[j], a, b, c)) {
                continue;
            }
            out.push([ia, ib, ic]);
            idx.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            // No ear (a self-crossing path): fan the rest so the area still shows.
            break;
        }
    }
    for k in 1..idx.len().saturating_sub(1) {
        out.push([idx[0], idx[k], idx[k + 1]]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    fn drag(points: &[(f32, f32)]) -> Lasso {
        let mut l = Lasso::new(pos2(points[0].0, points[0].1));
        for p in &points[1..] {
            l.add(pos2(p.0, p.1));
        }
        l
    }

    #[test]
    fn dragging_right_is_a_window_lasso_and_left_a_crossing_one() {
        assert_eq!(drag(&[(100.0, 100.0), (101.0, 90.0), (130.0, 80.0), (140.0, 120.0)]).kind(), LassoKind::Window);
        assert_eq!(drag(&[(100.0, 100.0), (99.0, 110.0), (60.0, 120.0), (50.0, 90.0)]).kind(), LassoKind::Crossing);
    }

    #[test]
    fn space_cycles_window_crossing_fence() {
        let mut l = drag(&[(0.0, 0.0), (20.0, 0.0), (20.0, 20.0)]);
        l.cycle();
        assert_eq!(l.kind(), LassoKind::Crossing);
        l.cycle();
        assert_eq!(l.kind(), LassoKind::Fence);
        l.cycle();
        assert_eq!(l.kind(), LassoKind::Window);
    }

    #[test]
    fn a_concave_lasso_is_covered_without_spilling_into_its_notch() {
        // An L shape: area 3 (in unit squares), not the 4 of its hull.
        let l = [pos2(0.0, 0.0), pos2(2.0, 0.0), pos2(2.0, 1.0), pos2(1.0, 1.0), pos2(1.0, 2.0), pos2(0.0, 2.0)];
        let tris = triangulate(&l);
        let area: f32 = tris.iter().map(|t| signed_area2(&[l[t[0]], l[t[1]], l[t[2]]]).abs() / 2.0).sum();
        assert!((area - 3.0).abs() < 1e-4, "{area}");
    }

    #[test]
    fn close_points_are_not_kept() {
        let l = drag(&[(0.0, 0.0), (0.5, 0.5), (1.0, 0.0), (5.0, 0.0)]);
        assert_eq!(l.points.len(), 2);
    }
}
