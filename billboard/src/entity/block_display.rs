//! [`BlockDisplay`]: v1's only entity — a client-side block display — with
//! its checkpoint state and the concrete weak-reference implementations.

use core::cell::Cell;
use core::marker::PhantomData;

use super::{BlockState, Dead, Entity, WeakMut, WeakRef, alive, over_ticks, sealed};
use crate::abi;
use crate::math::{Position, Rotation, Scale, Ticks};

/// A [`BlockDisplay`]'s complete visible state. A plain-data checkpoint:
/// extract it with [`BlockDisplay::state`], store it, tweak the fields, apply
/// it back with [`BlockDisplay::set`] / [`BlockDisplay::animate`].
#[derive(Clone, Debug, PartialEq)]
pub struct BlockDisplayState {
    pub block: BlockState,
    pub position: Position,
    pub rotation: Rotation,
    pub scale: Scale,
}

// --- Raw ABI helpers specific to a block display. The only place this
// entity's code touches pointers; the out-buffer unsafe for getters is
// contained here. ---

fn raw_set_position(id: i32, p: &Position, over: Ticks) {
    unsafe { abi::set_position(id, p.x, p.y, p.z, over_ticks(over)) }
}

fn raw_set_rotation(id: i32, r: &Rotation, over: Ticks) {
    unsafe { abi::set_rotation(id, r.x, r.y, r.z, r.w, over_ticks(over)) }
}

fn raw_set_scale(id: i32, s: &Scale, over: Ticks) {
    unsafe { abi::set_scale(id, s.x, s.y, s.z, over_ticks(over)) }
}

fn raw_set_block(id: i32, block: &BlockState) {
    let s = block.as_str();
    unsafe { abi::set_block(id, s.as_ptr(), s.len()) }
}

fn raw_get_position(id: i32) -> Position {
    let mut out = [0.0f64; 3];
    unsafe { abi::get_position(id, out.as_mut_ptr()) }
    Position::new(out[0], out[1], out[2])
}

fn raw_get_rotation(id: i32) -> Rotation {
    let mut out = [0.0f64; 4];
    unsafe { abi::get_rotation(id, out.as_mut_ptr()) }
    Rotation {
        x: out[0],
        y: out[1],
        z: out[2],
        w: out[3],
    }
}

fn raw_get_scale(id: i32) -> Scale {
    let mut out = [0.0f64; 3];
    unsafe { abi::get_scale(id, out.as_mut_ptr()) }
    Scale::new(out[0], out[1], out[2])
}

fn raw_get_block(id: i32) -> BlockState {
    // Two-call protocol: ask the length, then fill a buffer of that size.
    // No blocking point sits between the calls, so it can't race.
    let len = unsafe { abi::get_block_len(id) };
    let len = usize::try_from(len).expect("host returned a negative block-state length");
    let mut buf = vec![0u8; len];
    unsafe { abi::get_block(id, buf.as_mut_ptr()) }
    let s = String::from_utf8(buf).expect("host returned a non-UTF-8 block state");
    BlockState::new(s)
}

/// Apply a whole checkpoint via the per-attribute host calls. There is no
/// cache to diff against, so the block is always sent too.
fn raw_apply(id: i32, s: &BlockDisplayState, over: Ticks) {
    raw_set_position(id, &s.position, over);
    raw_set_rotation(id, &s.rotation, over);
    raw_set_scale(id, &s.scale, over);
    raw_set_block(id, &s.block);
}

fn raw_state(id: i32) -> BlockDisplayState {
    BlockDisplayState {
        block: raw_get_block(id),
        position: raw_get_position(id),
        rotation: raw_get_rotation(id),
        scale: raw_get_scale(id),
    }
}

/// The absolute owner of a client-side block display entity.
///
/// Move-only and `!Sync`: it cannot be captured by [`spawn`](crate::spawn)'s
/// closure, so ownership never leaves the task that created it. Dropping it
/// despawns the entity. To use the entity from another task, capture a
/// [`WeakRef`]/[`WeakMut`].
#[derive(Debug)]
pub struct BlockDisplay {
    id: i32,
    // Makes the owner !Sync (while staying Send): the compile-time guard
    // that keeps owner handles out of spawned tasks.
    _not_sync: PhantomData<Cell<()>>,
}

impl sealed::Sealed for BlockDisplay {}

impl Entity for BlockDisplay {
    type State = BlockDisplayState;
}

impl BlockDisplay {
    /// Spawn a block display at `position` (relative to the animation
    /// origin), unrotated and at scale 1.
    pub fn spawn(block: impl Into<BlockState>, position: impl AsRef<Position>) -> BlockDisplay {
        let block = block.into();
        let b = block.as_str();
        let p = position.as_ref();
        let id = unsafe { abi::spawn_block_display(b.as_ptr(), b.len(), p.x, p.y, p.z) };
        BlockDisplay {
            id,
            _not_sync: PhantomData,
        }
    }

    /// The entity's current state, freshly queried from the host.
    pub fn state(&self) -> BlockDisplayState {
        raw_state(self.id)
    }

    /// Apply a whole checkpoint instantly.
    pub fn set(&mut self, state: &BlockDisplayState) {
        raw_apply(self.id, state, Ticks::new(0));
    }

    /// Apply a whole checkpoint with client-side interpolation over `over`
    /// ticks. Non-blocking: returns immediately; `sleep` to wait it out. The
    /// block itself can't interpolate and switches instantly.
    pub fn animate(&mut self, state: &BlockDisplayState, over: Ticks) {
        raw_apply(self.id, state, over);
    }

    /// Current position, freshly queried from the host.
    pub fn position(&self) -> Position {
        raw_get_position(self.id)
    }

    /// Teleport instantly.
    pub fn teleport_to(&mut self, position: impl AsRef<Position>) {
        raw_set_position(self.id, position.as_ref(), Ticks::new(0));
    }

    /// Move to `position` over `over` ticks with interpolation (non-blocking).
    pub fn move_to(&mut self, position: impl AsRef<Position>, over: Ticks) {
        raw_set_position(self.id, position.as_ref(), over);
    }

    /// Current rotation, freshly queried from the host.
    pub fn rotation(&self) -> Rotation {
        raw_get_rotation(self.id)
    }

    /// Set the rotation instantly.
    pub fn set_rotation(&mut self, rotation: impl AsRef<Rotation>) {
        raw_set_rotation(self.id, rotation.as_ref(), Ticks::new(0));
    }

    /// Rotate over `over` ticks (non-blocking).
    pub fn rotate_to(&mut self, rotation: impl AsRef<Rotation>, over: Ticks) {
        raw_set_rotation(self.id, rotation.as_ref(), over);
    }

    /// Current scale, freshly queried from the host.
    pub fn scale(&self) -> Scale {
        raw_get_scale(self.id)
    }

    /// Set the scale instantly.
    pub fn set_scale(&mut self, scale: impl AsRef<Scale>) {
        raw_set_scale(self.id, scale.as_ref(), Ticks::new(0));
    }

    /// Rescale over `over` ticks (non-blocking).
    pub fn scale_to(&mut self, scale: impl AsRef<Scale>, over: Ticks) {
        raw_set_scale(self.id, scale.as_ref(), over);
    }

    /// The displayed block, freshly queried from the host.
    pub fn block(&self) -> BlockState {
        raw_get_block(self.id)
    }

    /// Swap the displayed block (instant — blocks can't interpolate).
    pub fn set_block(&mut self, block: impl Into<BlockState>) {
        raw_set_block(self.id, &block.into());
    }

    /// A read-only weak reference (aliveness + getters).
    pub fn weak(&self) -> WeakRef<BlockDisplay> {
        WeakRef::from_id(self.id)
    }

    /// A weak reference that can drive the entity but never kill it.
    pub fn weak_mut(&self) -> WeakMut<BlockDisplay> {
        WeakMut::from_id(self.id)
    }

    /// Despawn now. Equivalent to dropping the handle; reads better at the
    /// end of a scope.
    pub fn despawn(self) {
        // Drop does the work.
    }

    /// Give up ownership: the entity lives until the animation ends (the
    /// host despawns everything then). Returns a [`WeakMut`] to keep
    /// driving it.
    pub fn leak(self) -> WeakMut<BlockDisplay> {
        let weak = self.weak_mut();
        core::mem::forget(self);
        weak
    }
}

impl Drop for BlockDisplay {
    fn drop(&mut self) {
        // Unconditional: the owner can only ever be dropped in its own task
        // (it can't cross into a spawned closure), and a forked child never
        // unwinds the parent's frames — it runs its closure and exits.
        unsafe { abi::despawn(self.id) }
    }
}

impl WeakRef<BlockDisplay> {
    pub fn is_alive(&self) -> bool {
        alive(self.id())
    }

    fn check(&self) -> Result<(), Dead> {
        if self.is_alive() { Ok(()) } else { Err(Dead) }
    }

    /// The entity's current state, freshly queried from the host.
    pub fn state(&self) -> Result<BlockDisplayState, Dead> {
        self.check()?;
        Ok(raw_state(self.id()))
    }

    pub fn position(&self) -> Result<Position, Dead> {
        self.check()?;
        Ok(raw_get_position(self.id()))
    }

    pub fn rotation(&self) -> Result<Rotation, Dead> {
        self.check()?;
        Ok(raw_get_rotation(self.id()))
    }

    pub fn scale(&self) -> Result<Scale, Dead> {
        self.check()?;
        Ok(raw_get_scale(self.id()))
    }

    pub fn block(&self) -> Result<BlockState, Dead> {
        self.check()?;
        Ok(raw_get_block(self.id()))
    }
}

impl WeakMut<BlockDisplay> {
    pub fn is_alive(&self) -> bool {
        alive(self.id())
    }

    fn check(&self) -> Result<(), Dead> {
        if self.is_alive() { Ok(()) } else { Err(Dead) }
    }

    /// The entity's current state, freshly queried from the host.
    pub fn state(&self) -> Result<BlockDisplayState, Dead> {
        self.check()?;
        Ok(raw_state(self.id()))
    }

    pub fn position(&self) -> Result<Position, Dead> {
        self.check()?;
        Ok(raw_get_position(self.id()))
    }

    pub fn rotation(&self) -> Result<Rotation, Dead> {
        self.check()?;
        Ok(raw_get_rotation(self.id()))
    }

    pub fn scale(&self) -> Result<Scale, Dead> {
        self.check()?;
        Ok(raw_get_scale(self.id()))
    }

    pub fn block(&self) -> Result<BlockState, Dead> {
        self.check()?;
        Ok(raw_get_block(self.id()))
    }

    pub fn set(&mut self, state: &BlockDisplayState) -> Result<(), Dead> {
        self.check()?;
        raw_apply(self.id(), state, Ticks::new(0));
        Ok(())
    }

    pub fn animate(&mut self, state: &BlockDisplayState, over: Ticks) -> Result<(), Dead> {
        self.check()?;
        raw_apply(self.id(), state, over);
        Ok(())
    }

    pub fn teleport_to(&mut self, position: impl AsRef<Position>) -> Result<(), Dead> {
        self.check()?;
        raw_set_position(self.id(), position.as_ref(), Ticks::new(0));
        Ok(())
    }

    pub fn move_to(&mut self, position: impl AsRef<Position>, over: Ticks) -> Result<(), Dead> {
        self.check()?;
        raw_set_position(self.id(), position.as_ref(), over);
        Ok(())
    }

    pub fn set_rotation(&mut self, rotation: impl AsRef<Rotation>) -> Result<(), Dead> {
        self.check()?;
        raw_set_rotation(self.id(), rotation.as_ref(), Ticks::new(0));
        Ok(())
    }

    pub fn rotate_to(&mut self, rotation: impl AsRef<Rotation>, over: Ticks) -> Result<(), Dead> {
        self.check()?;
        raw_set_rotation(self.id(), rotation.as_ref(), over);
        Ok(())
    }

    pub fn set_scale(&mut self, scale: impl AsRef<Scale>) -> Result<(), Dead> {
        self.check()?;
        raw_set_scale(self.id(), scale.as_ref(), Ticks::new(0));
        Ok(())
    }

    pub fn scale_to(&mut self, scale: impl AsRef<Scale>, over: Ticks) -> Result<(), Dead> {
        self.check()?;
        raw_set_scale(self.id(), scale.as_ref(), over);
        Ok(())
    }

    pub fn set_block(&mut self, block: impl Into<BlockState>) -> Result<(), Dead> {
        self.check()?;
        raw_set_block(self.id(), &block.into());
        Ok(())
    }
}
