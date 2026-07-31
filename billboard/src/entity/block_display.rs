//! [`BlockDisplay`]: a client-side block display — a block, floating exactly
//! where you put it, with a full transform.
//!
//! # Geometry — the position is the block's *corner*
//!
//! A block display's position is the **minimum corner** of its model, not its
//! centre. At scale 1 the block fills the unit cube running from the position
//! towards +X, +Y and +Z — the same cube the real block with those coordinates
//! would occupy. Scale multiplies that cube about the same corner, so a
//! `Scale::splat(0.4)` tile spawned at `p` occupies `p ..= p + 0.4` on every
//! axis:
//!
//! ```text
//!        +Y
//!         │   ┌───────┐          scale 1: the cube [p, p+1]³
//!         │   │       │          scale s: the cube [p, p+s]³
//!         │   │       │
//!         p ──┴───────┴── +X     p is the low corner, never the middle
//!        ╱
//!      +Z
//! ```
//!
//! That is where hand-laid-out grids get their gap: tiles smaller than the cell
//! pitch hug the low corner of their cell and leave the slack at the top and on
//! the +X/+Z side, rather than splitting it evenly. To centre an `s`-sized tile
//! on a point, spawn it at `point - Offset::splat(s / 2.0)` — or let
//! [`Grid`](crate::helpers::Grid) do the arithmetic.
//!
//! The host adds nothing to this: it sends the display's transform translation
//! as `(0, 0, 0)` and never changes it, so what you get is vanilla
//! display-entity geometry with the placement origin added to your coordinates.
//!
//! # What the getters report mid-tween
//!
//! [`position`](BlockDisplay::position), [`rotation`](BlockDisplay::rotation),
//! [`scale`](BlockDisplay::scale) and [`state`](BlockDisplay::state) return the
//! **target** you last asked for, never an interpolated value. The host records
//! the target the instant you set it and leaves the interpolation to the
//! client, so `move_to(p, Ticks::new(40))` makes `position()` report `p` on the
//! very next line — 40 ticks before anything has visibly arrived. There is no
//! guest-side way to read the in-flight position; if you need to know where a
//! display *looks* like it is, compute it (an [`Ease`](crate::helpers::Ease)
//! over the same duration) rather than asking the host.
//!
//! # Accessor methods
//!
//! From the shared accessor macros: [`position`](BlockDisplay::position) /
//! [`teleport_to`](BlockDisplay::teleport_to) / [`move_to`](BlockDisplay::move_to),
//! [`rotation`](BlockDisplay::rotation) / [`set_rotation`](BlockDisplay::set_rotation) /
//! [`rotate_to`](BlockDisplay::rotate_to),
//! [`scale`](BlockDisplay::scale) / [`set_scale`](BlockDisplay::set_scale) /
//! [`scale_to`](BlockDisplay::scale_to),
//! [`billboard_mode`](BlockDisplay::billboard_mode) /
//! [`set_billboard_mode`](BlockDisplay::set_billboard_mode),
//! [`state`](BlockDisplay::state) / [`set`](BlockDisplay::set) /
//! [`animate`](BlockDisplay::animate),
//! [`weak`](BlockDisplay::weak) / [`weak_mut`](BlockDisplay::weak_mut) /
//! [`despawn`](BlockDisplay::despawn) / [`leak`](BlockDisplay::leak).
//! Block-display-specific: [`block`](BlockDisplay::block) /
//! [`set_block`](BlockDisplay::set_block). The weak references carry the same
//! set, each returning `Result<_, Dead>`.

use super::{BillboardMode, BlockState, Dead, WeakMut, WeakRef, raw};
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
    pub billboard_mode: BillboardMode,
}

/// Apply a whole checkpoint via the per-attribute host calls. There is no
/// cache to diff against, so the block is always sent too.
fn raw_apply(id: i32, s: &BlockDisplayState, over: Ticks) {
    raw::set_position(id, &s.position, over);
    raw::set_rotation(id, &s.rotation, over);
    raw::set_scale(id, &s.scale, over);
    raw::set_block(id, &s.block);
    raw::set_billboard_mode(id, s.billboard_mode);
}

fn raw_state(id: i32) -> BlockDisplayState {
    BlockDisplayState {
        block: raw::get_block(id),
        position: raw::get_position(id),
        rotation: raw::get_rotation(id),
        scale: raw::get_scale(id),
        billboard_mode: raw::get_billboard_mode(id),
    }
}

entity_handle! {
    /// The absolute owner of a client-side block display entity.
    ///
    /// Move-only and `!Sync`: it cannot be captured by
    /// [`spawn`](crate::spawn)'s closure, so ownership never leaves the task
    /// that created it. Dropping it despawns the entity. To use the entity
    /// from another task, capture a [`WeakRef`]/[`WeakMut`].
    BlockDisplay => BlockDisplayState
}

state_api!(owner BlockDisplay, BlockDisplayState);
state_api!(weak BlockDisplay, BlockDisplayState);
position_api!(owner BlockDisplay);
position_api!(weak BlockDisplay);
orientation_api!(owner BlockDisplay);
orientation_api!(weak BlockDisplay);
billboard_mode_api!(owner BlockDisplay);
billboard_mode_api!(weak BlockDisplay);

impl BlockDisplay {
    /// Spawn a block display at `position` (relative to the animation origin),
    /// unrotated, at scale 1, in [`BillboardMode::Fixed`].
    pub fn spawn(block: impl Into<BlockState>, position: impl AsRef<Position>) -> BlockDisplay {
        BlockDisplay::from_id(raw::spawn_block_display(&block.into(), position.as_ref()))
    }

    /// The displayed block, freshly queried from the host.
    pub fn block(&self) -> BlockState {
        raw::get_block(self.id)
    }

    /// Swap the displayed block (instant — blocks can't interpolate).
    pub fn set_block(&mut self, block: impl Into<BlockState>) {
        raw::set_block(self.id, &block.into());
    }
}

impl WeakRef<BlockDisplay> {
    pub fn block(&self) -> Result<BlockState, Dead> {
        self.check()?;
        Ok(raw::get_block(self.id()))
    }
}

impl WeakMut<BlockDisplay> {
    pub fn block(&self) -> Result<BlockState, Dead> {
        self.check()?;
        Ok(raw::get_block(self.id()))
    }

    pub fn set_block(&mut self, block: impl Into<BlockState>) -> Result<(), Dead> {
        self.check()?;
        raw::set_block(self.id(), &block.into());
        Ok(())
    }
}
