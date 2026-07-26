//! [`BlockDisplay`]: a client-side block display — a block, floating exactly
//! where you put it, with a full transform.

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
