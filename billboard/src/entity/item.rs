//! [`Item`]: a dropped item, faked with packets — so it can never be picked up.

use super::{ItemStr, raw};
use crate::math::{Position, Ticks};

/// An [`Item`]'s checkpoint state: what it is and where it is. That is all an
/// item entity has.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemState {
    pub item: ItemStr,
    pub position: Position,
}

fn raw_apply(id: i32, s: &ItemState, over: Ticks) {
    raw::set_position(id, &s.position, over);
    raw::set_item(id, &s.item);
}

fn raw_state(id: i32) -> ItemState {
    ItemState {
        item: raw::get_item(id),
        position: raw::get_position(id),
    }
}

entity_handle! {
    /// The absolute owner of a dropped-item entity.
    ///
    /// These are **packet-only**: the server has no entity for them at all, so
    /// they are inherently uncollectable and never despawn on their own — no
    /// pickup, no five-minute timer, no item merging. That is exactly what you
    /// want for scenery.
    ///
    /// An item has no rotation, no scale and no billboard mode (a dropped item
    /// bobs and spins on its own), so those methods deliberately **do not
    /// exist** — a compile error instead of a runtime kill. Position is
    /// tweened host-side, like an armor stand's.
    ///
    /// ```ignore
    /// let mut coin = Item::spawn(items::GOLD_INGOT, pos);
    /// coin.move_to(pos + Offset::new(0.0, 2.0, 0.0), Ticks::new(20));
    /// ```
    Item => ItemState
}

state_api!(owner Item, ItemState);
state_api!(weak Item, ItemState);
position_api!(owner Item);
position_api!(weak Item);
item_api!(owner Item);
item_api!(weak Item);

impl Item {
    /// Spawn a dropped item at `position`.
    pub fn spawn(item: impl Into<ItemStr>, position: impl AsRef<Position>) -> Item {
        Item::from_id(raw::spawn_item(&item.into(), position.as_ref()))
    }
}
