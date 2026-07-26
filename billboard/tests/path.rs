//! Known-answer tests for [`Path`]. Circle/arc expectations come from the
//! quarter-turn angles (cos/sin of 0, 90, 180°), bézier expectations from the
//! Bernstein form evaluated by hand at t = 1/2:
//! `B(½) = (p0 + 3p1 + 3p2 + p3)/8`.

use billboard::helpers::Path;
use billboard::math::{Degrees, Offset, Position, Vector3d};

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn assert_pos(got: Position, x: f64, y: f64, z: f64) {
    assert!(
        approx(got.x, x) && approx(got.y, y) && approx(got.z, z),
        "expected ({x}, {y}, {z}), got {got:?}"
    );
}

fn assert_off(got: Offset, x: f64, y: f64, z: f64) {
    assert!(
        approx(got.x, x) && approx(got.y, y) && approx(got.z, z),
        "expected ({x}, {y}, {z}), got {got:?}"
    );
}

#[test]
fn line_samples_and_tangent() {
    let p = Path::line(Position::new(0.0, 0.0, 0.0), Position::new(4.0, 2.0, -6.0));
    assert_pos(p.sample(0.0), 0.0, 0.0, 0.0);
    assert_pos(p.sample(0.5), 2.0, 1.0, -3.0);
    assert_pos(p.sample(1.0), 4.0, 2.0, -6.0);
    // Clamped, not extrapolated.
    assert_pos(p.sample(-1.0), 0.0, 0.0, 0.0);
    assert_pos(p.sample(3.0), 4.0, 2.0, -6.0);
    // A line's derivative is constant: the whole displacement.
    assert_off(p.tangent(0.0), 4.0, 2.0, -6.0);
    assert_off(p.tangent(1.0), 4.0, 2.0, -6.0);
}

#[test]
fn horizontal_circle_quarter_turns() {
    // normal = +Y: starts at +X and sweeps counter-clockwise seen from above,
    // which in right-handed coordinates takes +X towards -Z.
    let c = Path::circle(Position::new(1.0, 5.0, 2.0), 2.0, Vector3d::Y);
    assert_pos(c.sample(0.0), 3.0, 5.0, 2.0);
    assert_pos(c.sample(0.25), 1.0, 5.0, 0.0);
    assert_pos(c.sample(0.5), -1.0, 5.0, 2.0);
    assert_pos(c.sample(0.75), 1.0, 5.0, 4.0);
    assert_pos(c.sample(1.0), 3.0, 5.0, 2.0);
    // Constant radius all the way round.
    for i in 0..=8 {
        let d = c.sample(i as f64 / 8.0) - Position::new(1.0, 5.0, 2.0);
        assert!(approx(d.length(), 2.0), "radius drifted at step {i}");
    }
    // The tangent at t=0 points along the sweep (-Z), scaled by dθ/dt = 2π
    // and the radius.
    let tau = core::f64::consts::TAU;
    assert_off(c.tangent(0.0), 0.0, 0.0, -2.0 * tau);
    // Always perpendicular to the radius.
    let radius = c.sample(0.3) - Position::new(1.0, 5.0, 2.0);
    assert!(approx(radius.dot(c.tangent(0.3)), 0.0));
}

#[test]
fn vertical_circle_starts_at_plus_y() {
    // normal = +Z: the reference axis switches, and the circle starts at +Y.
    let c = Path::circle(Position::ZERO, 1.0, Vector3d::Z);
    assert_pos(c.sample(0.0), 0.0, 1.0, 0.0);
    assert_pos(c.sample(0.25), -1.0, 0.0, 0.0);
}

#[test]
fn arc_sweeps_the_requested_angles() {
    // A quarter turn of a horizontal circle, 0° to 90°.
    let a = Path::arc(Position::ZERO, 3.0, Vector3d::Y, 0.0, 90.0);
    assert_pos(a.sample(0.0), 3.0, 0.0, 0.0);
    assert_pos(a.sample(1.0), 0.0, 0.0, -3.0);
    // Halfway is 45°.
    let root_half = core::f64::consts::FRAC_1_SQRT_2;
    assert_pos(a.sample(0.5), 3.0 * root_half, 0.0, -3.0 * root_half);

    // Backwards sweep: 90° down to 0°.
    let b = Path::arc(
        Position::ZERO,
        3.0,
        Vector3d::Y,
        Degrees::new(90.0),
        Degrees::new(0.0),
    );
    assert_pos(b.sample(0.0), 0.0, 0.0, -3.0);
    assert_pos(b.sample(1.0), 3.0, 0.0, 0.0);
    // Reversed sweep, reversed tangent.
    assert_off(b.tangent(1.0), 0.0, 0.0, 3.0 * core::f64::consts::FRAC_PI_2);

    // An explicit basis gives an ellipse: 4 wide, 1 tall.
    let e = Path::arc_basis(
        Position::ZERO,
        Offset::new(4.0, 0.0, 0.0),
        Offset::new(0.0, 1.0, 0.0),
        0.0,
        core::f64::consts::FRAC_PI_2,
    );
    assert_pos(e.sample(0.0), 4.0, 0.0, 0.0);
    assert_pos(e.sample(1.0), 0.0, 1.0, 0.0);
}

#[test]
fn cubic_bezier_known_points() {
    // Collinear, evenly spaced control points: a straight line, and
    // B(½) = (0 + 3·1 + 3·2 + 3)/8 = 1.5.
    let straight = Path::cubic_bezier(
        Position::new(0.0, 0.0, 0.0),
        Position::new(1.0, 0.0, 0.0),
        Position::new(2.0, 0.0, 0.0),
        Position::new(3.0, 0.0, 0.0),
    );
    assert_pos(straight.sample(0.0), 0.0, 0.0, 0.0);
    assert_pos(straight.sample(0.5), 1.5, 0.0, 0.0);
    assert_pos(straight.sample(1.0), 3.0, 0.0, 0.0);
    // Derivative at ½: 3·¼·(1) + 6·¼·(1) + 3·¼·(1) = 3.
    assert_off(straight.tangent(0.5), 3.0, 0.0, 0.0);

    // An arch: B(½) x = (0 + 0 + 3 + 1)/8 = 0.5, y = (0 + 3 + 3 + 0)/8 = 0.75.
    let arch = Path::cubic_bezier(
        Position::new(0.0, 0.0, 0.0),
        Position::new(0.0, 1.0, 0.0),
        Position::new(1.0, 1.0, 0.0),
        Position::new(1.0, 0.0, 0.0),
    );
    assert_pos(arch.sample(0.5), 0.5, 0.75, 0.0);
    // Leaves p0 heading towards p1: 3·(p1 - p0) = (0, 3, 0).
    assert_off(arch.tangent(0.0), 0.0, 3.0, 0.0);
    // Arrives at p3 coming from p2: 3·(p3 - p2) = (0, -3, 0).
    assert_off(arch.tangent(1.0), 0.0, -3.0, 0.0);
}

#[test]
fn a_bezier_with_a_coincident_control_point_reports_a_zero_tangent() {
    // Not normalized, on purpose: there genuinely is no direction of travel
    // here, and inventing one would be a lie.
    let p = Path::cubic_bezier(
        Position::ZERO,
        Position::ZERO,
        Position::new(1.0, 0.0, 0.0),
        Position::new(1.0, 1.0, 0.0),
    );
    assert_off(p.tangent(0.0), 0.0, 0.0, 0.0);
}

#[test]
#[should_panic(expected = "non-finite")]
fn a_non_finite_path_sample_kills() {
    let p = Path::line(Position::ZERO, Position::new(1.0, 0.0, 0.0));
    let _ = p.sample(f64::NAN);
}

#[test]
#[should_panic(expected = "non-finite")]
fn a_non_finite_path_tangent_kills() {
    let p = Path::circle(Position::ZERO, 1.0, Vector3d::Y);
    let _ = p.tangent(f64::NAN);
}
