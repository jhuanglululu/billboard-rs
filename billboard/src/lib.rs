//! Billboard animation SDK.
//!
//! Write a Minecraft animation as a plain Rust program, compile it to
//! `wasm32-unknown-unknown`, drop it in the Billboard plugin's animations
//! folder. Everything is safe Rust: no raw pointers, no extern calls —
//! the ABI lives entirely inside this crate.
//!
//! ```ignore
//! use billboard::prelude::*;
//!
//! #[billboard::main]
//! fn main() -> ExitCode {
//!     let mut d = BlockDisplay::spawn("minecraft:sea_lantern", Position::ZERO);
//!     d.move_to(Position::new(0.0, 5.0, 0.0), Ticks::new(40));
//!     sleep(Ticks::new(40));
//!     ExitCode::End // drop despawns; the code tells the host how to clean up
//! }
//! ```

mod abi;
pub mod effects;
pub mod entity;
mod exit;
pub mod helpers;
pub mod math;
pub mod random;
pub mod registry;
pub mod sync;
mod task;

#[doc(hidden)]
#[path = "rt.rs"]
pub mod __rt;

/// Bumped whenever the guest ABI changes; `#[billboard::main]` exports it so
/// the plugin can refuse mismatched modules before running them.
///
/// Version 2 added the new entity kinds, sound and particles, the sync
/// primitives and the random streams — all additive, so the host accepts 1
/// and 2.
pub const ABI_VERSION: i32 = 2;

pub use billboard_macros::main;
pub use exit::ExitCode;
pub use task::{Task, sleep, spawn};

/// The `bytemuck` crate this SDK is built against, re-exported so an animation
/// can derive [`Pod`](bytemuck::Pod) without depending on bytemuck itself.
///
/// The derive macros expand to absolute `::bytemuck::…` paths, which do not
/// exist in an animation's crate graph, so a hand-written derive has to point
/// them here:
///
/// ```ignore
/// #[repr(C)]
/// #[derive(Clone, Copy, Pod, Zeroable)]
/// #[bytemuck(crate = "::billboard::bytemuck")]
/// struct Waypoint { target: Position, over: Ticks }
/// ```
///
/// [`payload!`](crate::payload) writes all of that for you, and is the way to do
/// it unless you need an unusual derive set.
pub use bytemuck;

/// Define a channel payload: a `#[repr(C)]` struct that is `Pod`, so its raw
/// bytes mean the same thing in another task's copy of memory.
///
/// ```ignore
/// billboard::payload! {
///     /// A waypoint handed to the runner task.
///     struct Waypoint {
///         target: Position,
///         over: Ticks,
///         sparkle: SplitRng,
///     }
/// }
///
/// let (tx, rx) = channel::<Waypoint>(4);
/// ```
///
/// Every field must itself be `Pod` — the SDK's math types, [`Color`], [`Ticks`]
/// and [`SplitRng`] all are. A `String`, a `Vec` or a reference is a compile
/// error, because the heap it points into does not exist in the receiving task.
/// Padding is rejected at compile time too, so order fields largest-first if the
/// derive complains.
///
/// Expands to `#[repr(C)]` plus `Clone, Copy, Debug, PartialEq, Pod, Zeroable`;
/// write the derive by hand (see [`bytemuck`]) if you need a different set.
///
/// [`Color`]: crate::helpers::Color
/// [`Ticks`]: crate::math::Ticks
/// [`SplitRng`]: crate::random::SplitRng
#[macro_export]
macro_rules! payload {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $($(#[$field_meta:meta])* $field_vis:vis $field:ident : $ty:ty),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[repr(C)]
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            $crate::bytemuck::Pod,
            $crate::bytemuck::Zeroable,
        )]
        // The derives emit absolute paths, so send them through this crate's
        // re-export and the animation needs no bytemuck dependency. Spelled out
        // rather than `$crate` because the attribute takes a string literal — if
        // you rename the `billboard` dependency, write the derive by hand with
        // your own path.
        #[bytemuck(crate = "::billboard::bytemuck")]
        $vis struct $name {
            $($(#[$field_meta])* $field_vis $field : $ty),*
        }
    };
}

/// Write a debug message to the server console.
pub fn log(msg: &str) {
    abi::marshal::log(msg);
}

pub mod prelude {
    pub use crate::effects::{Particle, SoundCategory, particle, sound};
    pub use crate::entity::{
        ArmorStand, ArmorStandState, BillboardMode, BlockDisplay, BlockDisplayState, BlockState,
        Dead, DisplayContext, Entity, EquipmentSlot, Item, ItemDisplay, ItemDisplayState,
        ItemState, ItemStr, Pose, PosePart, StandFlags, TextDisplay, TextDisplayState, TextFlags,
        WeakMut, WeakRef,
    };
    pub use crate::helpers::{
        Animate, BlockPalette, Color, Ease, Gradient, Group, Local, Oklab, Path, Timeline, Tween,
        text,
    };
    pub use crate::math::{
        Degrees, Offset, Position, Radians, Rotation, Scale, Ticks, Vector3d, Vector3i, Velocity,
    };
    pub use crate::random::{Rng, SplitRng, default_random};
    pub use crate::registry::{
        Axis, BlockId, BlockStateBuilder, Facing, Half, ItemId, blocks, items,
    };
    pub use crate::sync::{Barrier, Policy, Receiver, Sender, Signal, Waitable, channel};
    pub use crate::{ExitCode, Task, log, main, sleep, spawn};
    /// The channel-payload bound, and the derives that satisfy it.
    ///
    /// Reach for [`payload!`](crate::payload) to declare a payload struct — it
    /// applies these derives with the crate path they need. Deriving them by
    /// hand also works, with one extra line:
    /// `#[bytemuck(crate = "::billboard::bytemuck")]`, because the derive emits
    /// absolute `::bytemuck::…` paths and your animation has no bytemuck
    /// dependency of its own.
    pub use bytemuck::{Pod, Zeroable};
}
