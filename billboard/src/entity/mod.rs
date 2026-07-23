//! Entities, states, and the ownership model.
//!
//! Every entity has exactly one absolute owner (move-only struct; Drop
//! despawns). Owner handles are `!Sync`, so they cannot be captured by
//! `spawn`'s `Sync`-bounded closure — ownership provably never crosses into
//! another task, at compile time, with no runtime bookkeeping. Weak
//! references (`Sync + Clone`) are how other tasks observe or drive an
//! entity, and they are the only fallible APIs — everything else that goes
//! wrong kills the animation with a clear message.
//!
//! The SDK caches nothing: handles and weak refs hold only the entity id.
//! Every getter asks the host, the single source of truth across forked
//! memories. Applying a state issues the per-attribute host calls; there is
//! no cache to diff against, so the block is always resent too.
//!
//! Layout: this module holds the shared pieces (block-state strings, the
//! `Dead` error, the `Entity` trait, the generic weak-reference structs, and
//! the entity-agnostic raw helpers); each concrete entity lives in its own
//! submodule ([`block_display`] for v1's [`BlockDisplay`]).

use core::fmt;
use core::marker::PhantomData;

use crate::abi;
use crate::math::Ticks;

mod block_display;

pub use block_display::{BlockDisplay, BlockDisplayState};

/// A block state string, e.g. `"minecraft:oak_stairs[facing=east]"`.
/// Validated by the server on use; an invalid state kills the animation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockState(String);

impl BlockState {
    pub fn new(s: impl Into<String>) -> BlockState {
        BlockState(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for BlockState {
    fn from(s: &str) -> BlockState {
        BlockState(s.to_owned())
    }
}

impl From<String> for BlockState {
    fn from(s: String) -> BlockState {
        BlockState(s)
    }
}

impl fmt::Display for BlockState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The error weak references return once their entity is gone. The one
/// *expected* failure in the SDK — handle it or `.expect()` it; ignoring it
/// silently is the one thing you shouldn't do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dead;

impl fmt::Display for Dead {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the entity has been despawned")
    }
}

impl std::error::Error for Dead {}

mod sealed {
    pub trait Sealed {}
}

/// Implemented by every entity type; ties handles to their state struct.
pub trait Entity: sealed::Sealed {
    type State: Clone;
}

// --- Entity-agnostic raw ABI helpers. Shared by every entity module; the
// entity-typed getters/setters live alongside their entity. ---

/// Narrow an interpolation duration to the ABI's `i64`; overflow kills the
/// animation rather than wrapping.
fn over_ticks(over: Ticks) -> i64 {
    i64::try_from(over.count()).expect("interpolation duration overflows i64")
}

/// Host-truth liveness for a weak reference.
fn alive(id: i32) -> bool {
    unsafe { abi::is_alive(id) != 0 }
}

/// Read-only weak reference to an entity. Holds nothing but the entity id;
/// every read goes to the host. `Sync + Clone`; may outlive the owner, at
/// which point every method returns [`Dead`].
#[derive(Debug)]
pub struct WeakRef<T: Entity> {
    id: i32,
    // fn() -> T keeps this Sync/Send regardless of T (the handle type
    // itself is !Sync on purpose; the weak ref must not inherit that).
    _entity: PhantomData<fn() -> T>,
}

impl<T: Entity> WeakRef<T> {
    fn from_id(id: i32) -> WeakRef<T> {
        WeakRef {
            id,
            _entity: PhantomData,
        }
    }

    /// The entity id this reference observes.
    fn id(&self) -> i32 {
        self.id
    }
}

impl<T: Entity> Clone for WeakRef<T> {
    fn clone(&self) -> Self {
        WeakRef::from_id(self.id)
    }
}

/// Weak reference that can drive an entity (setters, apply states) but never
/// despawn it. Holds nothing but the entity id. `Sync + Clone`.
#[derive(Debug)]
pub struct WeakMut<T: Entity> {
    id: i32,
    _entity: PhantomData<fn() -> T>,
}

impl<T: Entity> WeakMut<T> {
    fn from_id(id: i32) -> WeakMut<T> {
        WeakMut {
            id,
            _entity: PhantomData,
        }
    }

    /// The entity id this reference drives.
    fn id(&self) -> i32 {
        self.id
    }
}

impl<T: Entity> Clone for WeakMut<T> {
    fn clone(&self) -> Self {
        WeakMut::from_id(self.id)
    }
}
