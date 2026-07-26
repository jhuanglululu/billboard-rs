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
//! no cache to diff against, so discrete fields (block, item, text) are always
//! resent too.
//!
//! # The entity kinds
//!
//! | Type | Looks like | Interpolation |
//! |---|---|---|
//! | [`BlockDisplay`] | a block | client-side |
//! | [`ItemDisplay`] | an item, held / dropped / in a GUI | client-side |
//! | [`TextDisplay`] | floating MiniMessage text | client-side |
//! | [`ArmorStand`] | a posable armor stand | host-side tween |
//! | [`Item`] | a dropped item | host-side tween |
//!
//! Displays interpolate client-side: one packet and the client does the rest.
//! Armor stands and items have no client interpolation at all, so the host
//! tweens them with per-tick packets — the SDK surface is identical
//! (`move_to(pos, ticks)` is still non-blocking and burns no guest fuel), and
//! their getters report the *target* value, consistent with displays.
//!
//! Layout: this module holds the shared pieces (identifier strings, the
//! attribute enums, the `Dead` error, the `Entity` trait, the generic weak
//! references); each entity kind lives in its own submodule, and every line of
//! pointer-touching ABI plumbing lives in `raw`.

use core::fmt;
use core::marker::PhantomData;

use crate::registry::{BlockId, ItemId};

#[macro_use]
mod accessors;

mod armor_stand;
mod block_display;
mod item;
mod item_display;
mod raw;
mod text_display;

pub use armor_stand::{ArmorStand, ArmorStandState, Pose};
pub use block_display::{BlockDisplay, BlockDisplayState};
pub use item::{Item, ItemState};
pub use item_display::{ItemDisplay, ItemDisplayState};
pub use text_display::{TextDisplay, TextDisplayState};

/// A block state string, e.g. `"minecraft:oak_stairs[facing=east]"`.
/// Validated by the server on use; an invalid state kills the animation.
///
/// Prefer the compile-time-checked [`blocks`](crate::registry::blocks) /
/// [`BlockStateBuilder`](crate::registry::BlockStateBuilder) route — both
/// convert straight into this type.
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

impl From<&BlockState> for BlockState {
    fn from(b: &BlockState) -> BlockState {
        b.clone()
    }
}

impl From<&BlockId> for BlockState {
    fn from(id: &BlockId) -> BlockState {
        BlockState::new(id.as_str())
    }
}

impl fmt::Display for BlockState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// An item, in the vanilla `/give` syntax: an id, optionally followed by
/// components —
/// `"minecraft:diamond_sword[minecraft:enchantment_glint_override=true]"`.
///
/// Parsed and validated by the server (Paper's `ItemFactory#createItemStack`)
/// **when it is used**, exactly as the `/give` command would; anything it
/// rejects kills the animation. [`ItemId`] converts in, so
/// `items::DIAMOND_SWORD` works wherever an item is wanted and has its id
/// checked at compile time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemStr(String);

impl ItemStr {
    pub fn new(s: impl Into<String>) -> ItemStr {
        ItemStr(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Append a component, in `/give` syntax:
    ///
    /// ```ignore
    /// items::PLAYER_HEAD
    ///     .into_item()
    ///     .with("minecraft:profile", "{name:'Notch'}")
    /// ```
    ///
    /// Components accumulate into the bracketed list; the server is what
    /// validates the result.
    pub fn with(self, component: &str, value: &str) -> ItemStr {
        let mut s = self.0;
        if s.ends_with(']') {
            // Already has a component list: splice into it.
            s.pop();
            s.push(',');
        } else {
            s.push('[');
        }
        s.push_str(component);
        s.push('=');
        s.push_str(value);
        s.push(']');
        ItemStr(s)
    }
}

impl From<&str> for ItemStr {
    fn from(s: &str) -> ItemStr {
        ItemStr(s.to_owned())
    }
}

impl From<String> for ItemStr {
    fn from(s: String) -> ItemStr {
        ItemStr(s)
    }
}

impl From<ItemId> for ItemStr {
    fn from(id: ItemId) -> ItemStr {
        ItemStr(id.as_str().to_owned())
    }
}

impl From<&ItemId> for ItemStr {
    fn from(id: &ItemId) -> ItemStr {
        ItemStr(id.as_str().to_owned())
    }
}

impl From<&ItemStr> for ItemStr {
    fn from(s: &ItemStr) -> ItemStr {
        s.clone()
    }
}

impl fmt::Display for ItemStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl ItemId {
    /// This id as an [`ItemStr`], ready for `.with(...)` components.
    pub fn into_item(self) -> ItemStr {
        ItemStr::from(self)
    }
}

/// How a display turns to face the viewer.
///
/// Wire values are vanilla's `Display$BillboardConstraints` ordinals:
/// `0 Fixed`, `1 Vertical`, `2 Horizontal`, `3 Center`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BillboardMode {
    /// No turning towards the viewer: the display keeps its own orientation.
    #[default]
    Fixed = 0,
    /// Turns around the vertical axis only.
    Vertical = 1,
    /// Turns around the horizontal axis only.
    Horizontal = 2,
    /// Always faces the viewer squarely.
    Center = 3,
}

impl BillboardMode {
    /// The ABI wire value.
    pub const fn wire(self) -> i32 {
        self as i32
    }

    /// Decode an ABI wire value. Public for the SDK's own wire-contract tests;
    /// an unknown value is an ABI mismatch and kills the animation rather than
    /// being guessed at.
    #[doc(hidden)]
    pub fn from_wire(v: i32) -> BillboardMode {
        match v {
            0 => BillboardMode::Fixed,
            1 => BillboardMode::Vertical,
            2 => BillboardMode::Horizontal,
            3 => BillboardMode::Center,
            other => panic!("host returned an unknown billboard mode: {other}"),
        }
    }
}

/// Which transform an [`ItemDisplay`] renders its item with — the same set of
/// "where is this item being shown" cases a vanilla item model has.
///
/// Wire values are vanilla's `ItemDisplayContext` ordinals (Bukkit's
/// `ItemDisplay.ItemDisplayTransform`), in declaration order: `0 None`,
/// `1 ThirdPersonLeftHand`, `2 ThirdPersonRightHand`, `3 FirstPersonLeftHand`,
/// `4 FirstPersonRightHand`, `5 Head`, `6 Gui`, `7 Ground`, `8 Fixed`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DisplayContext {
    /// No transform at all — the raw model.
    #[default]
    None = 0,
    ThirdPersonLeftHand = 1,
    ThirdPersonRightHand = 2,
    FirstPersonLeftHand = 3,
    FirstPersonRightHand = 4,
    /// As worn in a helmet slot.
    Head = 5,
    /// As drawn in an inventory slot — flat and front-on.
    Gui = 6,
    /// As a dropped item lying on the ground.
    Ground = 7,
    /// As an item frame shows it.
    Fixed = 8,
}

impl DisplayContext {
    /// The ABI wire value.
    pub const fn wire(self) -> i32 {
        self as i32
    }

    /// Decode an ABI wire value; see [`BillboardMode::from_wire`].
    #[doc(hidden)]
    pub fn from_wire(v: i32) -> DisplayContext {
        match v {
            0 => DisplayContext::None,
            1 => DisplayContext::ThirdPersonLeftHand,
            2 => DisplayContext::ThirdPersonRightHand,
            3 => DisplayContext::FirstPersonLeftHand,
            4 => DisplayContext::FirstPersonRightHand,
            5 => DisplayContext::Head,
            6 => DisplayContext::Gui,
            7 => DisplayContext::Ground,
            8 => DisplayContext::Fixed,
            other => panic!("host returned an unknown display context: {other}"),
        }
    }
}

/// A [`TextDisplay`]'s boolean options.
///
/// They travel as one ABI bitmask — `bit 0` shadow, `bit 1` see-through,
/// `bit 2` default background — matching vanilla's text-display flag byte.
/// The individual setters on [`TextDisplay`] read the current mask from the
/// host, flip their bit, and write it back, because the SDK keeps no cache.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextFlags {
    /// Draw a drop shadow behind the text.
    pub shadow: bool,
    /// Render through blocks.
    pub see_through: bool,
    /// Use the client's default background instead of this display's
    /// background colour.
    pub default_background: bool,
}

impl TextFlags {
    /// The ABI bitmask for these flags.
    pub const fn bits(self) -> i32 {
        (self.shadow as i32)
            | ((self.see_through as i32) << 1)
            | ((self.default_background as i32) << 2)
    }

    /// Unpack an ABI bitmask. Unknown high bits are ignored — they are the
    /// host's business, not an animation's.
    pub const fn from_bits(bits: i32) -> TextFlags {
        TextFlags {
            shadow: bits & 0b001 != 0,
            see_through: bits & 0b010 != 0,
            default_background: bits & 0b100 != 0,
        }
    }
}

/// An [`ArmorStand`]'s boolean options.
///
/// One ABI bitmask — `bit 0` small, `bit 1` arms, `bit 2` no base plate,
/// `bit 3` invisible. This is the *SDK/host* contract, not vanilla's on-wire
/// layout: vanilla splits these between the armor-stand data byte and the base
/// entity flags, and the host does that translation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct StandFlags {
    /// The small (baby-sized) armor stand.
    pub small: bool,
    /// Show arms.
    pub arms: bool,
    /// Hide the base plate.
    pub no_baseplate: bool,
    /// Hide the stand itself, leaving only its equipment visible.
    pub invisible: bool,
}

impl StandFlags {
    /// The ABI bitmask for these flags.
    pub const fn bits(self) -> i32 {
        (self.small as i32)
            | ((self.arms as i32) << 1)
            | ((self.no_baseplate as i32) << 2)
            | ((self.invisible as i32) << 3)
    }

    /// Unpack an ABI bitmask; unknown high bits are ignored.
    pub const fn from_bits(bits: i32) -> StandFlags {
        StandFlags {
            small: bits & 0b0001 != 0,
            arms: bits & 0b0010 != 0,
            no_baseplate: bits & 0b0100 != 0,
            invisible: bits & 0b1000 != 0,
        }
    }
}

/// Which limb of an [`ArmorStand`] a pose applies to. Wire values `0..=5`, in
/// the ABI's order: head, body, left arm, right arm, left leg, right leg.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PosePart {
    Head = 0,
    Body = 1,
    LeftArm = 2,
    RightArm = 3,
    LeftLeg = 4,
    RightLeg = 5,
}

impl PosePart {
    /// All six parts in wire order — for reading or writing a whole pose.
    pub const ALL: [PosePart; 6] = [
        PosePart::Head,
        PosePart::Body,
        PosePart::LeftArm,
        PosePart::RightArm,
        PosePart::LeftLeg,
        PosePart::RightLeg,
    ];

    /// The ABI wire value.
    pub const fn wire(self) -> i32 {
        self as i32
    }
}

/// Which equipment slot of an [`ArmorStand`] an item goes in. Wire values
/// `0..=5`: the four armor slots top-down, then the two hands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EquipmentSlot {
    Helmet = 0,
    Chestplate = 1,
    Leggings = 2,
    Boots = 3,
    MainHand = 4,
    OffHand = 5,
}

impl EquipmentSlot {
    /// All six slots in wire order.
    pub const ALL: [EquipmentSlot; 6] = [
        EquipmentSlot::Helmet,
        EquipmentSlot::Chestplate,
        EquipmentSlot::Leggings,
        EquipmentSlot::Boots,
        EquipmentSlot::MainHand,
        EquipmentSlot::OffHand,
    ];

    /// The ABI wire value.
    pub const fn wire(self) -> i32 {
        self as i32
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

    /// Whether the entity is still alive — asked of the host, always.
    pub fn is_alive(&self) -> bool {
        raw::alive(self.id)
    }

    fn check(&self) -> Result<(), Dead> {
        if self.is_alive() { Ok(()) } else { Err(Dead) }
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

    /// Whether the entity is still alive — asked of the host, always.
    pub fn is_alive(&self) -> bool {
        raw::alive(self.id)
    }

    fn check(&self) -> Result<(), Dead> {
        if self.is_alive() { Ok(()) } else { Err(Dead) }
    }

    /// A read-only view of the same entity.
    pub fn to_weak_ref(&self) -> WeakRef<T> {
        WeakRef::from_id(self.id)
    }
}

impl<T: Entity> Clone for WeakMut<T> {
    fn clone(&self) -> Self {
        WeakMut::from_id(self.id)
    }
}
