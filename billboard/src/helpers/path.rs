//! [`Path`]: parametric curves through space, for driving entities along a
//! route instead of hand-writing waypoints.
//!
//! Every path is parameterised by `t` in `0..=1`; [`sample`](Path::sample)
//! gives the point, [`tangent`](Path::tangent) the direction of travel there
//! (useful for making a display face along its own motion). `t` is *not*
//! arc length — a bézier moves faster where its control points pull it — which
//! is what you want for expressive motion and not what you want for constant
//! speed.

// Kernel sine/cosine rather than `f64::sin`/`f64::cos`: see `color.rs`.
use crate::math::{Degrees, Offset, Position, Radians, Vector3d, cos, sin};

/// A curve through space.
///
/// Circles and arcs are stored as a centre plus two in-plane basis offsets
/// (`u` at angle 0, `v` at angle 90°), which keeps sampling a plain
/// `centre + u·cos θ + v·sin θ` with no hidden plane conventions. The
/// [`circle`](Path::circle) / [`arc`](Path::arc) constructors build that basis
/// from a radius and a plane normal for you.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Path {
    /// Straight from `from` to `to`.
    Line { from: Position, to: Position },
    /// A full turn: `t` sweeps `0..2π` around `center`.
    Circle {
        center: Position,
        u: Offset,
        v: Offset,
    },
    /// A partial turn: `t` sweeps `start..end` (either direction).
    Arc {
        center: Position,
        u: Offset,
        v: Offset,
        start: Radians,
        end: Radians,
    },
    /// A cubic bézier: starts at `p0` heading towards `p1`, arrives at `p3`
    /// coming from `p2`.
    CubicBezier {
        p0: Position,
        p1: Position,
        p2: Position,
        p3: Position,
    },
}

impl Path {
    pub fn line(from: impl AsRef<Position>, to: impl AsRef<Position>) -> Path {
        Path::Line {
            from: *from.as_ref(),
            to: *to.as_ref(),
        }
    }

    /// A circle of `radius` around `center`, in the plane perpendicular to
    /// `normal`, swept counter-clockwise as seen from the `normal` side.
    pub fn circle(center: impl AsRef<Position>, radius: f64, normal: Vector3d) -> Path {
        let (u, v) = plane_basis(normal, radius);
        Path::Circle {
            center: *center.as_ref(),
            u,
            v,
        }
    }

    /// An arc of `radius` around `center` in the plane perpendicular to
    /// `normal`, from angle `start` to angle `end` (angle 0 is the direction
    /// [`circle`](Path::circle) starts in; `end` may be less than `start` to
    /// sweep backwards, or more than a full turn to spiral round).
    pub fn arc(
        center: impl AsRef<Position>,
        radius: f64,
        normal: Vector3d,
        start: impl Into<Degrees>,
        end: impl Into<Degrees>,
    ) -> Path {
        let (u, v) = plane_basis(normal, radius);
        Path::Arc {
            center: *center.as_ref(),
            u,
            v,
            start: Radians::from(start.into()),
            end: Radians::from(end.into()),
        }
    }

    /// An arc from an explicit basis: the point at angle `θ` is
    /// `center + u·cos θ + v·sin θ`, so non-perpendicular or unequal-length
    /// `u`/`v` give ellipses and skews on purpose.
    pub fn arc_basis(
        center: impl AsRef<Position>,
        u: Offset,
        v: Offset,
        start: impl Into<Radians>,
        end: impl Into<Radians>,
    ) -> Path {
        Path::Arc {
            center: *center.as_ref(),
            u,
            v,
            start: start.into(),
            end: end.into(),
        }
    }

    pub fn cubic_bezier(
        p0: impl AsRef<Position>,
        p1: impl AsRef<Position>,
        p2: impl AsRef<Position>,
        p3: impl AsRef<Position>,
    ) -> Path {
        Path::CubicBezier {
            p0: *p0.as_ref(),
            p1: *p1.as_ref(),
            p2: *p2.as_ref(),
            p3: *p3.as_ref(),
        }
    }

    /// The point at `t`, clamped to `0..=1`. A non-finite `t` kills — a NaN
    /// position would reach the host and draw nothing, silently.
    pub fn sample(&self, t: f64) -> Position {
        assert!(
            t.is_finite(),
            "Path::sample called with a non-finite t: {t}"
        );
        let t = t.clamp(0.0, 1.0);
        match *self {
            Path::Line { from, to } => from + (to - from) * t,
            Path::Circle { center, u, v } => on_circle(center, u, v, core::f64::consts::TAU * t),
            Path::Arc {
                center,
                u,
                v,
                start,
                end,
            } => on_circle(
                center,
                u,
                v,
                start.value() + (end.value() - start.value()) * t,
            ),
            Path::CubicBezier { p0, p1, p2, p3 } => {
                // De Casteljau in Bernstein form, on offsets from p0 so the
                // arithmetic stays inside the Position/Offset algebra.
                let (a, b, c) = (p1 - p0, p2 - p0, p3 - p0);
                let s = 1.0 - t;
                p0 + a * (3.0 * s * s * t) + b * (3.0 * s * t * t) + c * (t * t * t)
            }
        }
    }

    /// The derivative at `t` — direction of travel, magnitude proportional to
    /// speed in `t`. Deliberately **not** normalized: a bézier whose first two
    /// control points coincide has a genuinely zero derivative at `t = 0`, and
    /// silently inventing a direction there would be worse than reporting it.
    /// Normalize it yourself (`Vector3d::from(tangent).normalize()`) when you
    /// only want the heading.
    pub fn tangent(&self, t: f64) -> Offset {
        assert!(
            t.is_finite(),
            "Path::tangent called with a non-finite t: {t}"
        );
        let t = t.clamp(0.0, 1.0);
        match *self {
            Path::Line { from, to } => to - from,
            Path::Circle { center: _, u, v } => {
                let theta = core::f64::consts::TAU * t;
                (v * cos(theta) - u * sin(theta)) * core::f64::consts::TAU
            }
            Path::Arc {
                center: _,
                u,
                v,
                start,
                end,
            } => {
                let sweep = end.value() - start.value();
                let theta = start.value() + sweep * t;
                (v * cos(theta) - u * sin(theta)) * sweep
            }
            Path::CubicBezier { p0, p1, p2, p3 } => {
                // d/dt = 3(1-t)²(p1-p0) + 6(1-t)t(p2-p1) + 3t²(p3-p2)
                let s = 1.0 - t;
                (p1 - p0) * (3.0 * s * s) + (p2 - p1) * (6.0 * s * t) + (p3 - p2) * (3.0 * t * t)
            }
        }
    }
}

fn on_circle(center: Position, u: Offset, v: Offset, theta: f64) -> Position {
    center + u * cos(theta) + v * sin(theta)
}

/// Two perpendicular in-plane offsets of length `radius` for the plane whose
/// normal is `normal`.
///
/// The in-plane reference direction has to be picked from somewhere: take a
/// world axis that isn't nearly parallel to the normal (`+Z`, or `+X` when the
/// normal is itself near `±Z`) and cross with it. For the two cases that
/// actually come up this lands where you would expect — a horizontal circle
/// (`normal = +Y`) starts at `+X`, a circle facing the viewer (`normal = +Z`)
/// starts at `+Y` — and `v = normal × u` always makes the sweep
/// counter-clockwise seen from the normal side.
fn plane_basis(normal: Vector3d, radius: f64) -> (Offset, Offset) {
    let n = normal.normalize();
    let reference = if n.z.abs() < 0.9 {
        Vector3d::Z
    } else {
        Vector3d::X
    };
    let u = n.cross(reference).normalize();
    let v = n.cross(u);
    (Offset::from(u) * radius, Offset::from(v) * radius)
}
