//! Known-answer tests for the identifier registry: the ids the generated
//! snapshot must contain, and the exact string `BlockStateBuilder` renders.
//! Expected strings are written out by hand.

use billboard::entity::BlockState;
use billboard::registry::{Axis, BlockId, Facing, Half, blocks, items};

#[test]
fn ids_carry_their_namespaced_string() {
    assert_eq!(blocks::SEA_LANTERN.as_str(), "minecraft:sea_lantern");
    assert_eq!(blocks::RED_CONCRETE.as_str(), "minecraft:red_concrete");
    assert_eq!(items::DIAMOND_SWORD.as_str(), "minecraft:diamond_sword");
    // Const-constructible: usable in a const context, not just at runtime.
    const CUSTOM: BlockId = BlockId::new("mypack:neon_panel");
    assert_eq!(CUSTOM.as_str(), "mypack:neon_panel");
}

#[test]
fn a_bare_id_is_the_default_state() {
    let state: BlockState = blocks::SEA_LANTERN.into();
    assert_eq!(state.as_str(), "minecraft:sea_lantern");
}

#[test]
fn builder_renders_property_syntax() {
    // No properties: no brackets at all.
    assert_eq!(
        blocks::FURNACE.state().build().as_str(),
        "minecraft:furnace"
    );

    // One property.
    assert_eq!(
        blocks::FURNACE.state().lit(true).build().as_str(),
        "minecraft:furnace[lit=true]"
    );

    // Several: comma-separated, in the order they were set.
    assert_eq!(
        blocks::FURNACE
            .state()
            .facing(Facing::North)
            .lit(false)
            .build()
            .as_str(),
        "minecraft:furnace[facing=north,lit=false]"
    );

    assert_eq!(
        blocks::OAK_STAIRS
            .state()
            .half(Half::Top)
            .facing(Facing::East)
            .waterlogged(true)
            .build()
            .as_str(),
        "minecraft:oak_stairs[half=top,facing=east,waterlogged=true]"
    );

    assert_eq!(
        blocks::OAK_LOG.state().axis(Axis::Z).build().as_str(),
        "minecraft:oak_log[axis=z]"
    );

    assert_eq!(
        blocks::OAK_TRAPDOOR
            .state()
            .open(true)
            .powered(false)
            .build()
            .as_str(),
        "minecraft:oak_trapdoor[open=true,powered=false]"
    );
}

#[test]
fn builder_rotation_and_arbitrary_properties() {
    assert_eq!(
        blocks::OAK_SIGN.state().rotation(0).build().as_str(),
        "minecraft:oak_sign[rotation=0]"
    );
    assert_eq!(
        blocks::OAK_SIGN.state().rotation(15).build().as_str(),
        "minecraft:oak_sign[rotation=15]"
    );
    // The escape hatch: any property, as strings.
    assert_eq!(
        blocks::REPEATER
            .state()
            .with("delay", "3")
            .facing(Facing::West)
            .build()
            .as_str(),
        "minecraft:repeater[delay=3,facing=west]"
    );
}

#[test]
fn setting_a_property_twice_keeps_the_last_value_in_place() {
    // Position is the first-set position; the value is the last-set value.
    assert_eq!(
        blocks::FURNACE
            .state()
            .facing(Facing::North)
            .lit(true)
            .facing(Facing::South)
            .build()
            .as_str(),
        "minecraft:furnace[facing=south,lit=true]"
    );
}

#[test]
#[should_panic(expected = "rotation")]
fn rotation_above_fifteen_kills() {
    let _ = blocks::OAK_SIGN.state().rotation(16);
}

#[test]
fn builder_display_and_into_block_state_agree() {
    let b = blocks::FURNACE.state().facing(Facing::West).lit(true);
    let expected = "minecraft:furnace[facing=west,lit=true]";
    assert_eq!(b.to_string(), expected);
    assert_eq!(BlockState::from(&b).as_str(), expected);
    assert_eq!(BlockState::from(b).as_str(), expected);
}

#[test]
fn state_enums_render_vanilla_property_values() {
    assert_eq!(Facing::North.as_str(), "north");
    assert_eq!(Facing::East.as_str(), "east");
    assert_eq!(Facing::South.as_str(), "south");
    assert_eq!(Facing::West.as_str(), "west");
    assert_eq!(Facing::Up.as_str(), "up");
    assert_eq!(Facing::Down.as_str(), "down");
    assert_eq!(Axis::X.as_str(), "x");
    assert_eq!(Axis::Y.as_str(), "y");
    assert_eq!(Axis::Z.as_str(), "z");
    assert_eq!(Half::Top.as_str(), "top");
    assert_eq!(Half::Bottom.as_str(), "bottom");
}

/// The snapshot must cover the four 16-colour families `BlockPalette` indexes
/// and the blocks the demo animates, or the SDK doesn't build — this spells
/// out a sample of that contract in one readable place.
#[test]
fn snapshot_covers_the_palette_families_and_demo_blocks() {
    let ids = [
        blocks::WHITE_CONCRETE.as_str(),
        blocks::BLACK_CONCRETE.as_str(),
        blocks::LIGHT_BLUE_WOOL.as_str(),
        blocks::LIGHT_GRAY_TERRACOTTA.as_str(),
        blocks::MAGENTA_STAINED_GLASS.as_str(),
        blocks::GRAY_CONCRETE.as_str(),
        blocks::YELLOW_CONCRETE.as_str(),
        blocks::SEA_LANTERN.as_str(),
        blocks::EMERALD_BLOCK.as_str(),
        blocks::REDSTONE_BLOCK.as_str(),
    ];
    assert_eq!(
        ids,
        [
            "minecraft:white_concrete",
            "minecraft:black_concrete",
            "minecraft:light_blue_wool",
            "minecraft:light_gray_terracotta",
            "minecraft:magenta_stained_glass",
            "minecraft:gray_concrete",
            "minecraft:yellow_concrete",
            "minecraft:sea_lantern",
            "minecraft:emerald_block",
            "minecraft:redstone_block",
        ]
    );
}
