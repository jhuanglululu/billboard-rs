//! [`Ease`]: the standard easing curves, plus arbitrary cubic-bézier.
//!
//! An ease maps progress `t` in `0..=1` to eased progress, normally also in
//! `0..=1` (`Back` and `Elastic` deliberately overshoot). Minecraft's own
//! client-side interpolation is strictly linear, so easing an entity means
//! sub-stepping — feed the eased value into positions/scales yourself, or let
//! `Timeline` do it.
//!
//! Formulas are the conventional ones (the easings.net / Penner set) so the
//! curves feel like they do everywhere else.

use core::f64::consts::PI;

// The sine and elastic curves are the only transcendentals in the set, and they
// run once per sub-step of every eased tween — exactly the shape the kernel
// exists for. See `color.rs`.
use crate::math::{cos, pow, sin};

/// A named easing curve. [`apply`](Ease::apply) evaluates it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ease {
    /// No easing: `t`.
    Linear,
    QuadIn,
    QuadOut,
    QuadInOut,
    CubicIn,
    CubicOut,
    CubicInOut,
    SineIn,
    SineOut,
    SineInOut,
    /// Pulls back before moving.
    BackIn,
    /// Overshoots, then settles.
    BackOut,
    BackInOut,
    /// Winds up, then springs, oscillating.
    ElasticIn,
    ElasticOut,
    ElasticInOut,
    /// Bounces like a dropped ball.
    BounceIn,
    BounceOut,
    BounceInOut,
    /// An arbitrary CSS-style cubic bézier through `(0,0)` and `(1,1)` with
    /// control points `(x1, y1)` and `(x2, y2)`; `x1`/`x2` are clamped to
    /// `0..=1` so the curve stays a function of `t`.
    CubicBezier(f64, f64, f64, f64),
}

/// `Back`'s overshoot constant, and the two derived from it — the classic
/// Penner values.
const C1: f64 = 1.70158;
const C2: f64 = C1 * 1.525;
const C3: f64 = C1 + 1.0;
/// `Elastic`'s angular frequencies.
const E1: f64 = 2.0 * PI / 3.0;
const E2: f64 = 2.0 * PI / 4.5;

impl Ease {
    /// Eased progress for `t`, which is clamped to `0..=1` first. Every curve
    /// maps `0 → 0` and `1 → 1` exactly.
    ///
    /// A non-finite `t` kills the animation: clamping it would quietly turn a
    /// NaN that came from somewhere upstream into a plausible-looking `0.0`, and
    /// the curve would happily carry it into a position or a scale.
    pub fn apply(self, t: f64) -> f64 {
        assert!(t.is_finite(), "Ease::apply called with a non-finite t: {t}");
        let t = t.clamp(0.0, 1.0);
        match self {
            Ease::Linear => t,

            Ease::QuadIn => t * t,
            Ease::QuadOut => 1.0 - (1.0 - t) * (1.0 - t),
            Ease::QuadInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    let u = -2.0 * t + 2.0;
                    1.0 - u * u / 2.0
                }
            }

            Ease::CubicIn => t * t * t,
            Ease::CubicOut => {
                let u = 1.0 - t;
                1.0 - u * u * u
            }
            Ease::CubicInOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    let u = -2.0 * t + 2.0;
                    1.0 - u * u * u / 2.0
                }
            }

            Ease::SineIn => 1.0 - cos(t * PI / 2.0),
            Ease::SineOut => sin(t * PI / 2.0),
            Ease::SineInOut => -(cos(PI * t) - 1.0) / 2.0,

            Ease::BackIn => C3 * t * t * t - C1 * t * t,
            Ease::BackOut => {
                let u = t - 1.0;
                1.0 + C3 * u * u * u + C1 * u * u
            }
            Ease::BackInOut => {
                if t < 0.5 {
                    let u = 2.0 * t;
                    u * u * ((C2 + 1.0) * u - C2) / 2.0
                } else {
                    let u = 2.0 * t - 2.0;
                    (u * u * ((C2 + 1.0) * u + C2) + 2.0) / 2.0
                }
            }

            Ease::ElasticIn => {
                if t == 0.0 || t == 1.0 {
                    t
                } else {
                    -pow(2.0, 10.0 * t - 10.0) * sin((t * 10.0 - 10.75) * E1)
                }
            }
            Ease::ElasticOut => {
                if t == 0.0 || t == 1.0 {
                    t
                } else {
                    pow(2.0, -10.0 * t) * sin((t * 10.0 - 0.75) * E1) + 1.0
                }
            }
            Ease::ElasticInOut => {
                if t == 0.0 || t == 1.0 {
                    t
                } else if t < 0.5 {
                    -(pow(2.0, 20.0 * t - 10.0) * sin((20.0 * t - 11.125) * E2)) / 2.0
                } else {
                    pow(2.0, -20.0 * t + 10.0) * sin((20.0 * t - 11.125) * E2) / 2.0 + 1.0
                }
            }

            Ease::BounceIn => 1.0 - bounce_out(1.0 - t),
            Ease::BounceOut => bounce_out(t),
            Ease::BounceInOut => {
                if t < 0.5 {
                    (1.0 - bounce_out(1.0 - 2.0 * t)) / 2.0
                } else {
                    (1.0 + bounce_out(2.0 * t - 1.0)) / 2.0
                }
            }

            Ease::CubicBezier(x1, y1, x2, y2) => cubic_bezier(x1, y1, x2, y2, t),
        }
    }
}

/// The four-segment bounce, each segment a parabola with a smaller amplitude.
fn bounce_out(t: f64) -> f64 {
    const N: f64 = 7.5625;
    const D: f64 = 2.75;
    if t < 1.0 / D {
        N * t * t
    } else if t < 2.0 / D {
        let t = t - 1.5 / D;
        N * t * t + 0.75
    } else if t < 2.5 / D {
        let t = t - 2.25 / D;
        N * t * t + 0.9375
    } else {
        let t = t - 2.625 / D;
        N * t * t + 0.984375
    }
}

/// A cubic bézier from `(0,0)` to `(1,1)`. `t` is the *x* coordinate, so this
/// first solves `bezier_x(s) = t` for the curve parameter `s`, then returns
/// `bezier_y(s)` — the same two-step CSS timing functions do.
///
/// Newton's method converges in a couple of iterations for the well-behaved
/// control points animations use; the bisection fallback covers the rest, so
/// the solve always terminates.
fn cubic_bezier(x1: f64, y1: f64, x2: f64, y2: f64, t: f64) -> f64 {
    let x1 = x1.clamp(0.0, 1.0);
    let x2 = x2.clamp(0.0, 1.0);
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }

    // B(s) = 3(1-s)²s·p1 + 3(1-s)s²·p2 + s³, expanded in the Horner form
    // ((a·s + b)·s + c)·s with a = 1 - 3p2 + 3p1, b = 3p2 - 6p1, c = 3p1.
    let curve = |p1: f64, p2: f64, s: f64| {
        let a = 1.0 - 3.0 * p2 + 3.0 * p1;
        let b = 3.0 * p2 - 6.0 * p1;
        let c = 3.0 * p1;
        ((a * s + b) * s + c) * s
    };
    let slope = |p1: f64, p2: f64, s: f64| {
        let a = 1.0 - 3.0 * p2 + 3.0 * p1;
        let b = 3.0 * p2 - 6.0 * p1;
        let c = 3.0 * p1;
        (3.0 * a * s + 2.0 * b) * s + c
    };

    let mut s = t;
    for _ in 0..8 {
        let err = curve(x1, x2, s) - t;
        if err.abs() < 1e-9 {
            return curve(y1, y2, s);
        }
        let d = slope(x1, x2, s);
        if d.abs() < 1e-9 {
            break;
        }
        s -= err / d;
    }

    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    let mut s = t;
    for _ in 0..60 {
        let x = curve(x1, x2, s);
        if (x - t).abs() < 1e-12 {
            break;
        }
        if x < t {
            lo = s;
        } else {
            hi = s;
        }
        s = (lo + hi) / 2.0;
    }
    curve(y1, y2, s)
}
