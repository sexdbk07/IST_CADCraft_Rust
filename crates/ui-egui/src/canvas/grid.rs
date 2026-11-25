use super::draw::clip_infinite;
use super::{Bounds2, CadApp, Shape, Stroke, Tokens, Vec2, Xf};

/// Grid spacing adapted to the zoom: multiply the unit until lines are at least `min_px` apart.
fn adaptive(unit: f64, scale: f64, min_px: f64) -> f64 {
    let mut u = unit.max(1e-12);
    let mut guard = 0;
    while u * scale < min_px && guard < 40 {
        u *= 5.0;
        guard += 1;
    }
    u
}

/// iStCAD's grid step: lines about 20 pixels apart, the world step rounded to 1, 2, 5 or 10
/// times a power of ten (every fifth line is a major one).
fn istcad_step(scale: f64) -> f64 {
    let desired = 20.0 / scale.max(1e-12);
    let decade = 10f64.powf(desired.log10().floor());
    let n = desired / decade;
    let m = if n < 1.5 {
        1.0
    } else if n < 3.5 {
        2.0
    } else if n < 7.5 {
        5.0
    } else {
        10.0
    };
    m * decade
}

/// Index ranges of the grid lines inside `wb` for line spacing `minor`, or `None` when there would be
/// 2000 lines or more along an axis. The span is computed with checked arithmetic: a huge view
/// saturates the float-to-integer casts at the i64 limits, and subtracting those would overflow.
fn grid_index_ranges(wb: &Bounds2, minor: f64) -> Option<((i64, i64), (i64, i64))> {
    let range = |lo: f64, hi: f64| {
        let a = (lo / minor).floor() as i64;
        let b = (hi / minor).ceil() as i64;
        b.checked_sub(a).filter(|n| *n < 2000).map(|_| (a, b))
    };
    Some((range(wb.min.x, wb.max.x)?, range(wb.min.y, wb.max.y)?))
}

pub(super) fn draw_grid(app: &CadApp, p: &egui::Painter, xf: &Xf) {
    let t = Tokens::get();
    let s = app.session.drafting_settings();
    let wb = xf.world_bounds();
    if s.gridmode && s.isodraft {
        draw_iso_grid(app, p, xf);
    } else if s.gridmode {
        let major_n = f64::from(s.gridmajor.max(1));
        let unit = if s.gridunit.x == 0.0 { s.snapunit.x } else { s.gridunit.x };
        // The iStCAD shell draws iStCAD's grid: about 20 pixels, 1-2-5 steps, every fifth major.
        let (minor, major) = if app.istcad.enabled {
            let m = istcad_step(xf.scale);
            (m, m * 5.0)
        } else {
            let m = adaptive(unit, xf.scale, 8.0);
            (m, m * major_n)
        };
        // Keep the visible grid's checked ranges in the snap frame, then map its lines through
        // the UCS and the view (including its twist); the painter clips them to the viewport.
        let space = app.session.space();
        let (origin, angle) = app.session.doc().map(|d| cadcraft_engine::snap::grid_frame_in(d, &space)).unwrap_or((Vec2::ZERO, 0.0));
        let gb = Bounds2::from_points(wb.corners().map(|p| (p - origin).rotate(-angle)));
        let world = |p: Vec2| origin + p.rotate(angle);
        let mut lines = Vec::new();
        if let Some(((x0, x1), (y0, y1))) = grid_index_ranges(&gb, minor) {
            // Snap-frame lines across the visible box.
            for i in x0..=x1 {
                let x = i as f64 * minor;
                let is_major = (x / major).round() * major - x == 0.0 || ((x / major) - (x / major).round()).abs() < 1e-6;
                lines.push(Shape::line_segment(
                    [xf.to_screen(world(Vec2::new(x, gb.min.y))), xf.to_screen(world(Vec2::new(x, gb.max.y)))],
                    Stroke::new(1.0, if is_major { t.grid_major } else { t.grid_minor }),
                ));
            }
            for j in y0..=y1 {
                let y = j as f64 * minor;
                let is_major = ((y / major) - (y / major).round()).abs() < 1e-6;
                lines.push(Shape::line_segment(
                    [xf.to_screen(world(Vec2::new(gb.min.x, y))), xf.to_screen(world(Vec2::new(gb.max.x, y)))],
                    Stroke::new(1.0, if is_major { t.grid_major } else { t.grid_minor }),
                ));
            }
        }
        p.extend(lines);
    }
    // The current UCS's X and Y axes through its origin.
    let u = app.session.input_ucs();
    let o = u.origin.xy();
    let axis_w = if app.istcad.enabled { 1.15 } else { 1.0 };
    for (dir, color) in [(u.x.xy(), t.axis_x), (u.y.xy(), t.axis_y)] {
        if let Some(seg) = clip_infinite(xf, o, dir, false) {
            p.line_segment(seg, Stroke::new(axis_w, color));
        }
    }
}

/// The isometric grid (SNAPSTYL 1): lines at 30°, 90° and 150° through the snap lattice, GRIDUNIT
/// Y apart along the axes, coarsened while they would crowd closer than 8 pixels.
fn draw_iso_grid(app: &CadApp, p: &egui::Painter, xf: &Xf) {
    use cadcraft_engine::snap::iso;
    let t = Tokens::get();
    let s = app.session.drafting_settings();
    let mut d = s.gridunit.y.max(1e-12);
    for _ in 0..40 {
        if d * 0.866 * xf.scale >= 8.0 {
            break;
        }
        d *= 5.0;
    }
    // In the current UCS (SNAPBASE from its origin, SNAPANG from its X axis).
    let space = app.session.space();
    let (origin, angle) = app.session.doc().map(|d| cadcraft_engine::snap::grid_frame_in(d, &space)).unwrap_or((Vec2::ZERO, 0.0));
    let lines = iso::grid_lines(xf.world_bounds(), d, origin, angle, s.gridmajor);
    p.extend(
        lines.into_iter().map(|l| {
            Shape::line_segment([xf.to_screen(l.a), xf.to_screen(l.b)], Stroke::new(1.0, if l.major { t.grid_major } else { t.grid_minor }))
        }),
    );
}

#[cfg(test)]
mod grid_range_tests {
    use cadcraft_geom::{Bounds2, Vec2};

    #[test]
    fn grid_index_ranges_survive_extreme_views() {
        let huge = Bounds2::new(Vec2::new(-1e300, -1e300), Vec2::new(1e300, 1e300));
        assert_eq!(super::grid_index_ranges(&huge, 1.0), None);
        let normal = Bounds2::new(Vec2::new(-5.0, 0.0), Vec2::new(5.0, 10.0));
        assert_eq!(super::grid_index_ranges(&normal, 1.0), Some(((-5, 5), (0, 10))));
    }
}

#[cfg(test)]
mod istcad_grid_tests {
    use super::istcad_step;

    #[test]
    fn istcad_grid_lines_stay_near_twenty_pixels_on_a_1_2_5_ladder() {
        for scale in [0.013, 0.4, 1.0, 3.7, 25.0, 900.0] {
            let step = istcad_step(scale);
            let px = step * scale;
            assert!((9.0..=40.0).contains(&px), "scale {scale}: {px} px");
            let lead = step / 10f64.powf(step.log10().floor());
            assert!([1.0, 2.0, 5.0, 10.0].iter().any(|m| (lead - m).abs() < 1e-9), "{step}");
        }
    }
}
