//! Known-answer tests for [`Ease`]. Every expectation is hand-computed from
//! the standard formula for that curve — e.g. `BackIn(0.5) = 2.70158·0.125 −
//! 1.70158·0.25 = −0.0876975` — never read back off the implementation.

use billboard::helpers::Ease;

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

const ALL: [Ease; 20] = [
    Ease::Linear,
    Ease::QuadIn,
    Ease::QuadOut,
    Ease::QuadInOut,
    Ease::CubicIn,
    Ease::CubicOut,
    Ease::CubicInOut,
    Ease::SineIn,
    Ease::SineOut,
    Ease::SineInOut,
    Ease::BackIn,
    Ease::BackOut,
    Ease::BackInOut,
    Ease::ElasticIn,
    Ease::ElasticOut,
    Ease::ElasticInOut,
    Ease::BounceIn,
    Ease::BounceOut,
    Ease::BounceInOut,
    Ease::CubicBezier(0.42, 0.0, 0.58, 1.0),
];

#[test]
fn every_curve_pins_both_endpoints() {
    for ease in ALL {
        assert!(
            approx(ease.apply(0.0), 0.0),
            "{ease:?} at t=0 gave {}",
            ease.apply(0.0)
        );
        assert!(
            approx(ease.apply(1.0), 1.0),
            "{ease:?} at t=1 gave {}",
            ease.apply(1.0)
        );
    }
}

#[test]
fn t_is_clamped_not_extrapolated() {
    for ease in ALL {
        assert!(approx(ease.apply(-5.0), ease.apply(0.0)), "{ease:?}");
        assert!(approx(ease.apply(9.0), ease.apply(1.0)), "{ease:?}");
    }
}

#[test]
fn polynomial_curves() {
    assert!(approx(Ease::Linear.apply(0.25), 0.25));
    assert!(approx(Ease::Linear.apply(0.5), 0.5));

    // t²
    assert!(approx(Ease::QuadIn.apply(0.5), 0.25));
    // 1 - (1-t)²
    assert!(approx(Ease::QuadOut.apply(0.5), 0.75));
    // 2t² below the halfway point, mirrored above it.
    assert!(approx(Ease::QuadInOut.apply(0.25), 0.125));
    assert!(approx(Ease::QuadInOut.apply(0.5), 0.5));
    assert!(approx(Ease::QuadInOut.apply(0.75), 0.875));

    // t³
    assert!(approx(Ease::CubicIn.apply(0.5), 0.125));
    assert!(approx(Ease::CubicOut.apply(0.5), 0.875));
    // 4t³ below the halfway point.
    assert!(approx(Ease::CubicInOut.apply(0.25), 0.0625));
    assert!(approx(Ease::CubicInOut.apply(0.5), 0.5));
}

#[test]
fn sine_curves() {
    let root_half = core::f64::consts::FRAC_1_SQRT_2;
    // 1 - cos(45°)
    assert!(approx(Ease::SineIn.apply(0.5), 1.0 - root_half));
    // sin(45°)
    assert!(approx(Ease::SineOut.apply(0.5), root_half));
    // -(cos 90° - 1)/2
    assert!(approx(Ease::SineInOut.apply(0.5), 0.5));
}

#[test]
fn back_curves_overshoot_by_the_penner_constant() {
    // 2.70158·0.5³ - 1.70158·0.5² = 0.3376975 - 0.425395
    assert!(approx(Ease::BackIn.apply(0.5), -0.0876975));
    // Mirror image.
    assert!(approx(Ease::BackOut.apply(0.5), 1.0876975));
    // The in-out crossover is exactly halfway.
    assert!(approx(Ease::BackInOut.apply(0.5), 0.5));
    // BackIn dips below zero early on, BackOut overshoots late.
    assert!(Ease::BackIn.apply(0.2) < 0.0);
    assert!(Ease::BackOut.apply(0.8) > 1.0);
}

#[test]
fn elastic_curves() {
    // ElasticIn(0.5) = -2^-5 · sin((5 - 10.75)·2π/3); the sine argument is
    // -π/6 modulo 2π, so sin = 0.5 and the result is -0.03125·0.5.
    assert!(approx(Ease::ElasticIn.apply(0.5), -0.015625));
    // ElasticOut(0.5) = 2^-5 · sin((5 - 0.75)·2π/3) + 1, and that sine is
    // sin(5π/6) = 0.5.
    assert!(approx(Ease::ElasticOut.apply(0.5), 1.015625));
    // ElasticInOut's midpoint: 2^0 · sin(-π/2)/2 + 1.
    assert!(approx(Ease::ElasticInOut.apply(0.5), 0.5));
}

#[test]
fn bounce_curves() {
    // Second parabola of BounceOut: 7.5625·(0.5 - 1.5/2.75)² + 0.75.
    assert!(approx(Ease::BounceOut.apply(0.5), 0.765625));
    // BounceIn is BounceOut reflected through both axes.
    assert!(approx(Ease::BounceIn.apply(0.5), 1.0 - 0.765625));
    // BounceInOut's midpoint is (1 + BounceOut(0))/2.
    assert!(approx(Ease::BounceInOut.apply(0.5), 0.5));
    // First parabola: 7.5625·0.1² = 0.075625.
    assert!(approx(Ease::BounceOut.apply(0.1), 0.075625));
}

#[test]
fn cubic_bezier_identities() {
    // Control points on the diagonal make the curve y = x exactly, for both
    // the (1/3, 2/3) and the (0, 1) parameterisations.
    let diagonal = Ease::CubicBezier(1.0 / 3.0, 1.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0);
    let corners = Ease::CubicBezier(0.0, 0.0, 1.0, 1.0);
    for t in [0.0, 0.1, 0.25, 0.5, 0.75, 0.9, 1.0] {
        assert!(
            (diagonal.apply(t) - t).abs() < 1e-6,
            "diagonal bezier at {t} gave {}",
            diagonal.apply(t)
        );
        assert!(
            (corners.apply(t) - t).abs() < 1e-6,
            "corner bezier at {t} gave {}",
            corners.apply(t)
        );
    }
}

#[test]
fn cubic_bezier_is_monotone_for_css_ease() {
    // CSS `ease` = cubic-bezier(0.25, 0.1, 0.25, 1). It must stay inside
    // 0..1 and never go backwards.
    let css = Ease::CubicBezier(0.25, 0.1, 0.25, 1.0);
    let mut previous = 0.0;
    for i in 0..=100 {
        let y = css.apply(i as f64 / 100.0);
        assert!((0.0..=1.0).contains(&y), "css ease left 0..1 at {i}: {y}");
        assert!(y >= previous - 1e-9, "css ease went backwards at {i}");
        previous = y;
    }
    // It starts faster than linear (that is the whole point of `ease`).
    assert!(css.apply(0.25) > 0.25);
}

#[test]
#[should_panic(expected = "non-finite")]
fn a_non_finite_ease_input_kills() {
    // NaN would propagate straight through the curve and out into a position.
    let _ = Ease::CubicInOut.apply(f64::NAN);
}

#[test]
#[should_panic(expected = "non-finite")]
fn an_infinite_ease_input_kills() {
    let _ = Ease::Linear.apply(f64::NEG_INFINITY);
}
