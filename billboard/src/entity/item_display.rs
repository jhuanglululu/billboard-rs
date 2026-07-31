//! [`ItemDisplay`]: a client-side item display — any item model, floating with
//! a full transform, rendered as if it were held, worn, dropped or in a GUI.
//!
//! # Geometry
//!
//! An item display's item model is drawn **centred on the position** (the
//! [`DisplayContext`] chooses which of vanilla's item transforms is applied on
//! top of that, which is what makes a `Head` sword sit differently from a
//! `Gui` one). Unlike a [`BlockDisplay`](crate::entity::BlockDisplay), whose
//! position is its low corner, there is no half-block offset to compensate for
//! here. The host contributes nothing either way: the transform translation is
//! sent as `(0, 0, 0)` and never changed.
//!
//! Getters report the **target**, not an interpolated value: right after
//! `move_to(p, over)`, `position()` is already `p`.
//!
//! # Accessor methods
//!
//! [`position`](ItemDisplay::position) / [`teleport_to`](ItemDisplay::teleport_to) /
//! [`move_to`](ItemDisplay::move_to),
//! [`rotation`](ItemDisplay::rotation) / [`set_rotation`](ItemDisplay::set_rotation) /
//! [`rotate_to`](ItemDisplay::rotate_to),
//! [`scale`](ItemDisplay::scale) / [`set_scale`](ItemDisplay::set_scale) /
//! [`scale_to`](ItemDisplay::scale_to),
//! [`billboard_mode`](ItemDisplay::billboard_mode) /
//! [`set_billboard_mode`](ItemDisplay::set_billboard_mode),
//! [`item`](ItemDisplay::item) / [`set_item`](ItemDisplay::set_item),
//! [`context`](ItemDisplay::context) / [`set_context`](ItemDisplay::set_context),
//! [`state`](ItemDisplay::state) / [`set`](ItemDisplay::set) /
//! [`animate`](ItemDisplay::animate),
//! [`weak`](ItemDisplay::weak) / [`weak_mut`](ItemDisplay::weak_mut) /
//! [`despawn`](ItemDisplay::despawn) / [`leak`](ItemDisplay::leak). The weak
//! references carry the same set, each returning `Result<_, Dead>`.

use super::{BillboardMode, Dead, DisplayContext, ItemStr, WeakMut, WeakRef, raw};
use crate::math::{Position, Rotation, Scale, Ticks};

/// An [`ItemDisplay`]'s complete visible state — a plain-data checkpoint.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemDisplayState {
    /// The item, in `/give` syntax. Validated by the server when applied.
    pub item: ItemStr,
    /// Which item transform the model is drawn with.
    pub context: DisplayContext,
    pub position: Position,
    pub rotation: Rotation,
    pub scale: Scale,
    pub billboard_mode: BillboardMode,
}

fn raw_apply(id: i32, s: &ItemDisplayState, over: Ticks) {
    raw::set_position(id, &s.position, over);
    raw::set_rotation(id, &s.rotation, over);
    raw::set_scale(id, &s.scale, over);
    raw::set_item(id, &s.item);
    raw::set_display_context(id, s.context);
    raw::set_billboard_mode(id, s.billboard_mode);
}

fn raw_state(id: i32) -> ItemDisplayState {
    ItemDisplayState {
        item: raw::get_item(id),
        context: raw::get_display_context(id),
        position: raw::get_position(id),
        rotation: raw::get_rotation(id),
        scale: raw::get_scale(id),
        billboard_mode: raw::get_billboard_mode(id),
    }
}

entity_handle! {
    /// The absolute owner of a client-side item display entity.
    ///
    /// Same ownership rules as every entity: move-only, `!Sync`, Drop
    /// despawns; cross tasks with [`WeakRef`]/[`WeakMut`].
    ///
    /// ```ignore
    /// let mut sword = ItemDisplay::spawn(items::DIAMOND_SWORD, pos);
    /// sword.set_context(DisplayContext::ThirdPersonRightHand);
    /// sword.rotate_to(&Rotation::axis_angle(Vector3d::Y, Degrees::new(180.0)), Ticks::new(20));
    /// ```
    ItemDisplay => ItemDisplayState
}

state_api!(owner ItemDisplay, ItemDisplayState);
state_api!(weak ItemDisplay, ItemDisplayState);
position_api!(owner ItemDisplay);
position_api!(weak ItemDisplay);
orientation_api!(owner ItemDisplay);
orientation_api!(weak ItemDisplay);
billboard_mode_api!(owner ItemDisplay);
billboard_mode_api!(weak ItemDisplay);
item_api!(owner ItemDisplay);
item_api!(weak ItemDisplay);

impl ItemDisplay {
    /// Spawn an item display at `position`, unrotated, at scale 1, with
    /// [`DisplayContext::None`] and [`BillboardMode::Fixed`].
    ///
    /// `item` is anything that converts into [`ItemStr`]: an
    /// [`ItemId`](crate::registry::ItemId) from the generated registry (id
    /// checked at compile time), or a full `/give`-syntax string with
    /// components. The server validates it here, and rejects kill the
    /// animation.
    pub fn spawn(item: impl Into<ItemStr>, position: impl AsRef<Position>) -> ItemDisplay {
        ItemDisplay::from_id(raw::spawn_item_display(&item.into(), position.as_ref()))
    }

    /// Which item transform the model is drawn with, freshly queried from the
    /// host.
    pub fn context(&self) -> DisplayContext {
        raw::get_display_context(self.id)
    }

    /// Set the item transform (instant — a context can't interpolate).
    pub fn set_context(&mut self, context: DisplayContext) {
        raw::set_display_context(self.id, context);
    }
}

impl WeakRef<ItemDisplay> {
    pub fn context(&self) -> Result<DisplayContext, Dead> {
        self.check()?;
        Ok(raw::get_display_context(self.id()))
    }
}

impl WeakMut<ItemDisplay> {
    pub fn context(&self) -> Result<DisplayContext, Dead> {
        self.check()?;
        Ok(raw::get_display_context(self.id()))
    }

    pub fn set_context(&mut self, context: DisplayContext) -> Result<(), Dead> {
        self.check()?;
        raw::set_display_context(self.id(), context);
        Ok(())
    }
}
