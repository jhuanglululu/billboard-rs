//! The QoL layer: pure guest Rust, almost no ABI traffic, zero host code.
//!
//! Everything here is the kind of thing an animation would otherwise rewrite
//! from scratch — perceptual colour blending, picking a block that matches a
//! colour, easing curves, curves through space, moving a pile of entities as
//! one. Most of it composes with the entity API rather than wrapping it: an
//! [`Ease`] gives you a number, a [`Path`] gives you a
//! [`Position`](crate::math::Position), and you decide what to do with them.
//! [`Group`], [`Timeline`] and the [`text`] effects do drive entities, through
//! exactly the same public handles an animation would use.
//!
//! - [`Color`] / [`Oklab`] / [`Gradient`] — colour, blended perceptually.
//! - [`BlockPalette`] — nearest block to a colour.
//! - [`Ease`] — the standard easing curves.
//! - [`Path`] — lines, circles, arcs, béziers.
//! - [`Group`] — many entities moved as one rigid assembly.
//! - [`Timeline`] — keyframed states with per-segment easing.
//! - [`text`] — typewriter and marquee effects for text displays.

mod color;
mod ease;
mod group;
mod palette;
mod path;
pub mod text;
mod timeline;

pub use color::{Color, Gradient, Oklab};
pub use ease::Ease;
pub use group::{DeadMembers, Group, GroupMember, Local};
pub use palette::BlockPalette;
pub use path::Path;
pub use timeline::{Animate, DEFAULT_SUB_STEP, Step, Timeline, Tween};
