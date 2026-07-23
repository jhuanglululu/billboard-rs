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
//! fn main() {
//!     let mut d = BlockDisplay::spawn("minecraft:sea_lantern", Position::ZERO);
//!     d.move_to(Position::new(0.0, 5.0, 0.0), Ticks::new(40));
//!     sleep(Ticks::new(40));
//! } // drop despawns; main returning ends the animation
//! ```

mod abi;
pub mod entity;
pub mod math;
mod task;

#[doc(hidden)]
#[path = "rt.rs"]
pub mod __rt;

/// Bumped whenever the guest ABI changes; `#[billboard::main]` exports it so
/// the plugin can refuse mismatched modules before running them.
pub const ABI_VERSION: i32 = 1;

pub use billboard_macros::main;
pub use task::{Task, sleep, spawn};

/// Write a debug message to the server console.
pub fn log(msg: &str) {
    unsafe { abi::log(msg.as_ptr(), msg.len()) }
}

pub mod prelude {
    pub use crate::entity::{
        BlockDisplay, BlockDisplayState, BlockState, Dead, Entity, WeakMut, WeakRef,
    };
    pub use crate::math::{
        Degrees, Offset, Position, Radians, Rotation, Scale, Ticks, Vector3d, Vector3i, Velocity,
    };
    pub use crate::{Task, log, main, sleep, spawn};
}
