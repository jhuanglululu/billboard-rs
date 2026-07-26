//! Block and item identifiers, and the typed block-state builder.
//!
//! The plugin — not this crate — is the source of truth for what exists in a
//! given server. `/billboard export registry` writes Rust source: a const per
//! block id, a const per item id, and the common block-state enums. The SDK's
//! `build.rs` copies that file (`$BILLBOARD_REGISTRY`, else the bundled
//! `registry-snapshot.rs`) into `OUT_DIR`, and the [`include!`] below pulls it
//! straight into this module, so **rustc is the validator**: a typo'd block
//! name is a compile error naming the missing const, not a runtime kill.
//!
//! ```ignore
//! use billboard::prelude::*;
//!
//! BlockDisplay::spawn(blocks::SEA_LANTERN, Position::ZERO);
//! BlockDisplay::spawn(blocks::FURNACE.state().facing(Facing::North).lit(true), pos);
//! BlockDisplay::spawn(blocks::REPEATER.state().with("delay", "3"), pos);
//! ```
//!
//! What is *not* checked here: whether this block actually has that property.
//! Per-block typed builders are deliberately out of scope — the server
//! validates the finished string when the state is used, and an invalid one
//! kills the animation (error philosophy). Ids and common property values are
//! compile-time; the combination is server-checked.
//!
//! Sounds have no consts and are never validated: resolution is client-side
//! and resource packs extend the namespace. That is the SDK's one documented
//! exception to the error philosophy.

use core::fmt;

use crate::entity::BlockState;

// The generated registry: `blocks`, `items`, and the common-state enums
// (`Axis`, `Facing`, `Half`, …). See registry-snapshot.rs for the format
// contract the plugin's exporter must satisfy.
include!(concat!(env!("OUT_DIR"), "/registry.rs"));

/// A block identifier, e.g. `minecraft:sea_lantern`. Const-constructible; the
/// generated [`blocks`] module is a const of these per block the server knows.
///
/// A bare `BlockId` converts straight into a [`BlockState`] (the default state
/// of that block); call [`BlockId::state`] to set properties first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockId(&'static str);

impl BlockId {
    /// Wrap an identifier. Used by the generated registry; call it yourself
    /// only for an id the export doesn't have (the server validates it at
    /// use, like any state string).
    pub const fn new(id: &'static str) -> BlockId {
        BlockId(id)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }

    /// Start building a block state with properties:
    /// `blocks::OAK_STAIRS.state().facing(Facing::East).half(Half::Top)`.
    pub fn state(self) -> BlockStateBuilder {
        BlockStateBuilder {
            id: self,
            props: Vec::new(),
        }
    }
}

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl From<BlockId> for BlockState {
    fn from(id: BlockId) -> BlockState {
        BlockState::new(id.0)
    }
}

/// An item identifier, e.g. `minecraft:diamond_sword`. Const-constructible;
/// the generated [`items`] module is a const of these per item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemId(&'static str);

impl ItemId {
    /// Wrap an identifier. Used by the generated registry.
    pub const fn new(id: &'static str) -> ItemId {
        ItemId(id)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// Builds the `id[key=value,…]` string form of a block state.
///
/// Handwritten (not generated): typed setters for the properties that come up
/// constantly, [`with`](BlockStateBuilder::with) for everything else. Setting
/// the same property twice keeps the last value; properties render in the
/// order they were first set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockStateBuilder {
    id: BlockId,
    props: Vec<(String, String)>,
}

/// `rotation` values as static strings — a 16-entry table instead of integer
/// formatting, which keeps `format!` machinery out of the wasm module.
const ROTATIONS: [&str; 16] = [
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
];

impl BlockStateBuilder {
    /// Set (or overwrite) a property.
    fn put(mut self, key: &str, value: &str) -> BlockStateBuilder {
        match self.props.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = value.to_owned(),
            None => self.props.push((key.to_owned(), value.to_owned())),
        }
        self
    }

    /// `facing=north` — stairs, furnaces, observers, most directional blocks.
    pub fn facing(self, facing: Facing) -> BlockStateBuilder {
        self.put("facing", facing.as_str())
    }

    /// `axis=y` — logs, pillars, chains.
    pub fn axis(self, axis: Axis) -> BlockStateBuilder {
        self.put("axis", axis.as_str())
    }

    /// `half=top` — stairs, trapdoors, doors.
    pub fn half(self, half: Half) -> BlockStateBuilder {
        self.put("half", half.as_str())
    }

    /// `lit=true` — furnaces, campfires, redstone lamps.
    pub fn lit(self, lit: bool) -> BlockStateBuilder {
        self.put("lit", bool_str(lit))
    }

    /// `open=true` — doors, trapdoors, fence gates.
    pub fn open(self, open: bool) -> BlockStateBuilder {
        self.put("open", bool_str(open))
    }

    /// `powered=true` — redstone-reactive blocks.
    pub fn powered(self, powered: bool) -> BlockStateBuilder {
        self.put("powered", bool_str(powered))
    }

    /// `waterlogged=true`.
    pub fn waterlogged(self, waterlogged: bool) -> BlockStateBuilder {
        self.put("waterlogged", bool_str(waterlogged))
    }

    /// `rotation=0..15` — signs, banners, skulls. A value above 15 is a bug
    /// and kills the animation.
    pub fn rotation(self, rotation: u8) -> BlockStateBuilder {
        assert!(
            rotation <= 15,
            "block-state `rotation` must be 0..=15, got {rotation}"
        );
        self.put("rotation", ROTATIONS[rotation as usize])
    }

    /// Any other property, as raw strings: `.with("delay", "3")`. The server
    /// validates it when the state is used.
    pub fn with(self, key: impl AsRef<str>, value: impl AsRef<str>) -> BlockStateBuilder {
        self.put(key.as_ref(), value.as_ref())
    }

    /// The block id this state is built on.
    pub fn id(&self) -> BlockId {
        self.id
    }

    /// Render to the `id[k=v,…]` string form. `BlockStateBuilder` also
    /// converts into [`BlockState`], so entity setters take it directly.
    pub fn build(&self) -> BlockState {
        BlockState::new(self.render())
    }

    /// Built by hand rather than with `format!`, so a block-heavy animation
    /// doesn't drag the formatting machinery into its wasm module.
    fn render(&self) -> String {
        if self.props.is_empty() {
            return self.id.0.to_owned();
        }
        let len = self.id.0.len()
            + 2
            + self
                .props
                .iter()
                .map(|(k, v)| k.len() + v.len() + 2)
                .sum::<usize>();
        let mut s = String::with_capacity(len);
        s.push_str(self.id.0);
        s.push('[');
        for (i, (k, v)) in self.props.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(k);
            s.push('=');
            s.push_str(v);
        }
        s.push(']');
        s
    }
}

const fn bool_str(v: bool) -> &'static str {
    if v { "true" } else { "false" }
}

impl fmt::Display for BlockStateBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl From<BlockStateBuilder> for BlockState {
    fn from(b: BlockStateBuilder) -> BlockState {
        b.build()
    }
}

impl From<&BlockStateBuilder> for BlockState {
    fn from(b: &BlockStateBuilder) -> BlockState {
        b.build()
    }
}
