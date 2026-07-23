//! [`Rotation`]: an orientation stored as a unit quaternion.

use super::angle::Radians;
use super::vectors::Vector3d;

/// An orientation, stored as a quaternion. Build one with
/// [`Rotation::axis_angle`] or [`Rotation::euler`]; compose with `*`
/// (right-hand side applies first).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rotation {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

impl Rotation {
    pub const IDENTITY: Rotation = Rotation {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };

    /// Rotation of `angle` around `axis`. The axis needs a nonzero length —
    /// a zero axis is a bug and kills the animation.
    pub fn axis_angle(axis: Vector3d, angle: impl Into<Radians>) -> Rotation {
        let len = (axis.x * axis.x + axis.y * axis.y + axis.z * axis.z).sqrt();
        assert!(len > 0.0, "Rotation::axis_angle requires a nonzero axis");
        let a = angle.into().value();
        let (s, c) = (a / 2.0).sin_cos();
        Rotation {
            x: axis.x / len * s,
            y: axis.y / len * s,
            z: axis.z / len * s,
            w: c,
        }
    }

    /// Yaw (around +Y), then pitch (around +X), then roll (around +Z).
    pub fn euler(
        yaw: impl Into<Radians>,
        pitch: impl Into<Radians>,
        roll: impl Into<Radians>,
    ) -> Rotation {
        Rotation::axis_angle(Vector3d::Y, yaw)
            * Rotation::axis_angle(Vector3d::X, pitch)
            * Rotation::axis_angle(Vector3d::Z, roll)
    }
}

impl Default for Rotation {
    fn default() -> Rotation {
        Rotation::IDENTITY
    }
}

impl core::ops::Mul for Rotation {
    type Output = Rotation;
    fn mul(self, r: Rotation) -> Rotation {
        Rotation {
            w: self.w * r.w - self.x * r.x - self.y * r.y - self.z * r.z,
            x: self.w * r.x + self.x * r.w + self.y * r.z - self.z * r.y,
            y: self.w * r.y - self.x * r.z + self.y * r.w + self.z * r.x,
            z: self.w * r.z + self.x * r.y - self.y * r.x + self.z * r.w,
        }
    }
}

/// So a shared `&Rotation` can be handed to entity setters just like the
/// macro-generated vector types (which get this impl from `vectors!`).
impl AsRef<Rotation> for Rotation {
    fn as_ref(&self) -> &Rotation {
        self
    }
}

// Raw-quaternion conversions, in (x, y, z, w) order — the same tuple/array
// round-tripping the vector family gets from `vectors!`. These are for
// callers who already hold quaternion components; they are *not* normalized,
// matching the "explicit opt-out of the type discipline" rule.
impl From<(f64, f64, f64, f64)> for Rotation {
    fn from(q: (f64, f64, f64, f64)) -> Rotation {
        Rotation {
            x: q.0,
            y: q.1,
            z: q.2,
            w: q.3,
        }
    }
}

impl From<[f64; 4]> for Rotation {
    fn from(q: [f64; 4]) -> Rotation {
        Rotation {
            x: q[0],
            y: q[1],
            z: q[2],
            w: q[3],
        }
    }
}

impl From<Rotation> for (f64, f64, f64, f64) {
    fn from(r: Rotation) -> (f64, f64, f64, f64) {
        (r.x, r.y, r.z, r.w)
    }
}

impl From<Rotation> for [f64; 4] {
    fn from(r: Rotation) -> [f64; 4] {
        [r.x, r.y, r.z, r.w]
    }
}
