//! Object snaps on what a command has placed but not yet added to the drawing
//! (`Interactive::snap_objects`): a LEADER's points snap while it is still being drawn, as the
//! segments a PLINE has drawn do.

use cadcraft_engine::Session;
use cadcraft_engine::geom::Vec2;
use cadcraft_engine::snap::mode;
use cadcraft_engine::snap::tracking::CursorQuery;

fn snapped(s: &mut Session, raw: Vec2, base: Option<Vec2>) -> Option<(u32, Vec2)> {
    let q = CursorQuery { base, deferred: false, corner: false, aperture: 0.5, tol: 0.1, now: 100.0, shift: false };
    s.snap_cursor(raw, &q).snap.map(|h| (h.mode, h.point))
}

#[test]
fn leader_points_snap_before_the_leader_is_added() {
    let mut s = Session::new();
    s.settings.osmode = mode::END | mode::MID;
    let before = s.doc().unwrap().model.len();
    s.cmdline("leader 0,0 10,0 10,10").unwrap();
    assert_eq!(s.doc().unwrap().model.len(), before, "nothing added yet");
    let base = Some(Vec2::new(10.0, 10.0));
    // The midpoint of the first leg and the arrow end, though neither is in the drawing.
    let hit = snapped(&mut s, Vec2::new(5.1, 0.2), base);
    assert!(hit.is_some_and(|(m, p)| m == mode::MID && p.near(Vec2::new(5.0, 0.0), 1e-9)), "{hit:?}");
    let hit = snapped(&mut s, Vec2::new(0.2, -0.1), base);
    assert!(hit.is_some_and(|(m, p)| m == mode::END && p.near(Vec2::ZERO, 1e-9)), "{hit:?}");
    // Once the command ends, the drawing is searched as it is.
    s.cancel();
    assert!(snapped(&mut s, Vec2::new(5.1, 0.2), None).is_none());
}
