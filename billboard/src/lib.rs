//! Billboard animation SDK.
//!
//! Write a Minecraft animation as a plain Rust program, compile it to
//! `wasm32-unknown-unknown`, drop it in the Billboard plugin's animations
//! folder. Everything is safe Rust: no raw pointers, no extern calls — the ABI
//! lives inside this crate's `abi` module and inside the `wasmachine` core it
//! is built on, and nowhere else.
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
//!
//! # The coordinate frame you are writing in
//!
//! Every [`Position`](math::Position) an animation hands the host is
//! **origin-relative**: the plugin maps your coordinates into the world and
//! sends the result. For an unrotated placement (the default) that map is a
//! pure translation — the plugin adds the placement's `x/y/z` and the axes are
//! the world's axes, no matter where the admin stood when they typed the
//! command. A placement *may* carry a rotation (`/billboard spawn … <x> <y>
//! <z> [yaw] [pitch] [roll]`, vanilla `/tp` conventions): the whole frame then
//! turns rigidly around the origin, host-side — positions, tweens, particles,
//! sounds and display orientations all follow, and the animation itself never
//! sees it. Author in the local frame below and let the admin aim the show.
//!
//! ```text
//!            +Y  up
//!             │
//!             │
//!             O ────── +X  east          O = Position::ZERO
//!            ╱                             = the placement's x y z
//!          +Z  south
//! ```
//!
//! - `+X` is world east, `+Z` is world south, `+Y` is up — vanilla's axes.
//! - **The assumed viewer stands at `−Z` and looks towards `+Z`** (facing
//!   south, the northern side of the scene). Nothing in the plugin enforces
//!   that — players walk where they like — but it is the direction the SDK's
//!   helpers are written for, and the one that makes "left" and "right"
//!   mean anything: it is why [`Grid`](helpers::Grid) puts column 0 on the
//!   left and rows running down, and why a flat scene built in the XY plane
//!   reads the right way round rather than mirrored. Build the front of a
//!   scene facing `−Z`.
//! - `Position::ZERO` is the exact point in `/billboard spawn <animation> <id>
//!   <x> <y> <z> …`, which is a plain coordinate triple, *not* the admin's
//!   position and not snapped to a block.
//! - Nothing is oriented towards the viewer. A sign built in the XY plane faces
//!   north/south; to face it another way, rotate it yourself (
//!   [`set_rotation`](entity::BlockDisplay::set_rotation), or a
//!   [`Group`](helpers::Group) turned as one), or give text and item displays a
//!   [`BillboardMode`](entity::BillboardMode) and let the client turn them.
//! - The same relative coordinates are used by sounds and particles, so a
//!   `Position` means one thing everywhere in the SDK.
//!
//! Consequence worth internalising: two placements of one animation are the
//! same scene in two spots, and an animation that hardcodes `y = 3.0` is three
//! blocks above whatever `y` the placement was given.
//!
//! # What the host gives you per tick
//!
//! These are the plugin's **defaults**, from its shipped `config.toml`; an
//! operator can change them, so treat them as the budget you should fit inside
//! comfortably rather than a constant to compute against.
//!
//! - **Instructions: 1,000,000 per instance per game tick**, shared by every
//!   task. Overrunning it is not throttling — the animation is killed and
//!   paused, loudly. A million interpreted instructions is a lot of arithmetic
//!   and a poor budget for, say, converting a whole colour table per lookup
//!   (see [`BlockPalette`](helpers::BlockPalette)); host calls are cheap in
//!   *this* budget but are real packets, which is the other number to watch.
//! - **Memory: 16 MiB per instance**, and **every task fork copies the whole
//!   private memory**, so a three-task animation can cost three times that.
//!   Channel buffers count towards it too. [`env`]'s key/value pairs do not —
//!   they live in a host-shared region outside the per-task copy, referenced
//!   rather than duplicated by every fork.
//! - **Ticks are 50 ms** and the interpreter runs on a worker pool, never the
//!   main thread — a slow tick of yours costs you, not the server's TPS.
//! - **Audience: a 64-block radius** around the placement origin, by default.
//!   An instance only runs while an eligible player is inside it, and it *dies*
//!   (rather than pausing) once they leave and the linger window expires — the
//!   next approach starts a fresh run from the top. Build scenes that read from
//!   inside that radius, and do not assume an animation ever gets to finish.
//! - **Environ: a fixed set of string key/value pairs**, snapshotted once when
//!   the instance starts. See [`env`] below.
//!
//! ## How many host calls a tick is too many?
//!
//! The honest answer is that you are almost certainly asking about the wrong
//! budget. Worked through, for the case that prompts the question — a 176-cell
//! panel spawned in a single tick, which is two calls per cell (spawn, then
//! scale) plus a handful of labels, so roughly **350 host calls**:
//!
//! - **Against the instruction budget: trivial.** A host call crossing costs
//!   *tens* of interpreted instructions (the engine's ABI design notes give
//!   that figure while arguing why transcendentals deserve a math kernel and
//!   plain arithmetic does not). Even rounding a crossing up to a hundred,
//!   350 of them is ~35,000 of 1,000,000 — a few per cent, before the
//!   arithmetic that computed the arguments. You would have to be making tens
//!   of thousands of calls in one tick for the crossings themselves to be the
//!   thing that kills you. Note this is a design-doc figure, not one measured
//!   from this SDK.
//! - **Against the client: this is the number that matters.** Every one of
//!   those calls becomes a packet *per viewer*, immediately — the renderer
//!   sends on the calling thread, with no queue, no coalescing and no rate
//!   limit anywhere in the plugin. A spawn is two packets (spawn + metadata),
//!   each attribute one more, so that 350-call burst is ~530 packets per
//!   viewer in one tick, and ~2,600 with five people watching. The bill is
//!   `entities × changes-per-tick × viewers` and nothing on the host side
//!   flattens the spike for you.
//!
//! So: hundreds of host calls in a tick is fine, and hundreds *every* tick is
//! the thing to design away from. The levers are the ones the SDK already
//! gives you — interpolate instead of stepping (`move_to`/`animate`/`pulse`
//! with a duration is one packet for a whole movement), touch a slice rather
//! than a sheet ([`Grid::fill_row`](helpers::Grid::fill_row),
//! [`pulse_row`](helpers::Grid::pulse_row)), and stagger a big spawn across a
//! few ticks with `sleep` when the burst is large enough to see.
//!
//! ## Entities
//!
//! There is **no hard cap** in the plugin: it never refuses a spawn, and there
//! is no per-instance entity limit anywhere in its code. What there is instead:
//! every entity is a client-side fake, so each one costs a spawn packet plus a
//! metadata packet per viewer, and every attribute you set costs one more
//! packet per viewer. The bill scales with `entities × changes-per-tick ×
//! viewers`, and that product — not a limit — is what eventually hurts.
//!
//! For a reference point: `demo/src/lib.rs`, the SDK's worked example, holds
//! about **50** entities at once (a 15-block panel, a 16-tile colour strip, a
//! five-part logo group and a handful of performers), and its busiest stretch
//! re-blocks all 16 strip tiles every other tick. That runs comfortably. Grids in
//! the low hundreds are fine if you touch a slice of them per tick rather than
//! all of them; a thousand entities all animating every tick is a packet
//! firehose, and interpolated `move_to`/`animate` (one packet, client-side
//! interpolation) is how you avoid needing to.
//!
//! # Environ
//!
//! [`env`] is a per-placement, read-only set of string key/value pairs a
//! server operator sets with `/billboard env` — animation-scoped defaults,
//! placement-scoped overrides on top of them, and a handful of host built-ins
//! (`bb.x`/`bb.y`/`bb.z`, `bb.type`, and `bb.player` in a per-player
//! instance). It is snapshotted once when the instance starts and **fixed for
//! the whole run**: an operator's edit takes effect on the next fresh
//! instance, never on one already running. There is no way to be notified of
//! a change and nothing to invalidate — a value read on tick 1 is still good
//! on tick 10,000.
//!
//! Because an animation runs from whatever placements exist, and a placement
//! may set nothing at all, treat every key as optional and give it a sensible
//! default in code rather than assuming an operator configured it:
//!
//! ```ignore
//! let speed: f64 = env::get("speed")
//!     .and_then(|s| s.parse().ok())
//!     .unwrap_or(1.0);
//! ```

mod abi;
pub mod effects;
pub mod entity;
mod exit;
pub mod helpers;
pub mod registry;

// The generic guest core — tasks, sync, randomness, math, the panic hook and
// the host allocator — lives in `wasmachine`, shared with any other plugin
// built on the same engine. Re-exported here so animation code sees one SDK:
// `billboard::math::Position`, `billboard::sync::Signal`, and so on.
pub use wasmachine::{env, math, random, sync};

#[doc(hidden)]
pub use wasmachine::__rt;

/// Bumped whenever Billboard's half of the guest ABI changes;
/// `#[billboard::main]` exports it as `_billboard_abi` so the plugin can refuse
/// mismatched modules before running them.
///
/// Version 2 added the new entity kinds, sound and particles, the sync
/// primitives and the random streams — all additive, so the host accepted 1
/// and 2. Version 3 is the namespace split: the engine's imports (memory,
/// tasks, sync, random, math) moved to module `"engine"` and the entry point
/// became `_engine_main`, leaving module `"billboard"` to the entity and effect
/// imports alone. Nothing in this module's own list changed — but a v2 guest
/// asks the host for engine functions under the plugin's name, so the two
/// cannot be mixed.
pub const ABI_VERSION: i32 = 3;

/// The engine ABI the SDK is built against, re-exported so
/// `#[billboard::main]`'s generated `_engine_abi` export can reach it through
/// this crate — an animation depends on `billboard`, never on `wasmachine`.
pub use wasmachine::ENGINE_ABI_VERSION;

pub use billboard_macros::main;
pub use exit::ExitCode;
pub use wasmachine::{Task, log, sleep, spawn};

/// The entry-point machinery `#[billboard::main]` expands into, re-exported so
/// the generated attribute resolves inside an animation's own crate (which
/// depends on `billboard` alone). Not for animations to name.
#[doc(hidden)]
pub use wasmachine_macros::sdk_main as __sdk_main;

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

pub mod prelude {
    pub use crate::effects::{Particle, SoundCategory, particle, sound};
    pub use crate::entity::{
        ArmorStand, ArmorStandState, BillboardMode, BlockDisplay, BlockDisplayState, BlockState,
        Dead, DisplayContext, Entity, EquipmentSlot, Item, ItemDisplay, ItemDisplayState,
        ItemState, ItemStr, Pose, PosePart, StandFlags, TextDisplay, TextDisplayState, TextFlags,
        WeakMut, WeakRef,
    };
    pub use crate::helpers::{
        Animate, BlockPalette, Color, Ease, Gradient, Grid, GridLayout, Group, Local, Oklab, Path,
        Timeline, Tween, text,
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
