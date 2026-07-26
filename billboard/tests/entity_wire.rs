//! The ABI wire contract for the v2 entity attributes: every ordinal and every
//! flag bit written out by hand, so a renumbering has to argue with a test.
//!
//! These numbers are not free choices — the display context, billboard mode and
//! sound category ordinals must match vanilla's own enums (the host passes them
//! straight through to packets), and the flag layouts must match what the plugin
//! decodes.

use billboard::effects::SoundCategory;
use billboard::entity::{
    BillboardMode, BlockState, DisplayContext, EquipmentSlot, ItemStr, PosePart, StandFlags,
    TextFlags,
};
use billboard::helpers::Color;
use billboard::registry::{ItemId, blocks, items};

#[test]
fn billboard_mode_ordinals() {
    // vanilla Display$BillboardConstraints: FIXED, VERTICAL, HORIZONTAL, CENTER
    assert_eq!(BillboardMode::Fixed as i32, 0);
    assert_eq!(BillboardMode::Vertical as i32, 1);
    assert_eq!(BillboardMode::Horizontal as i32, 2);
    assert_eq!(BillboardMode::Center as i32, 3);
    // The unset default is Fixed: a display keeps the orientation you gave it.
    assert_eq!(BillboardMode::default(), BillboardMode::Fixed);
}

#[test]
fn display_context_ordinals() {
    // vanilla ItemDisplayContext / Bukkit ItemDisplay.ItemDisplayTransform, in
    // declaration order.
    assert_eq!(DisplayContext::None as i32, 0);
    assert_eq!(DisplayContext::ThirdPersonLeftHand as i32, 1);
    assert_eq!(DisplayContext::ThirdPersonRightHand as i32, 2);
    assert_eq!(DisplayContext::FirstPersonLeftHand as i32, 3);
    assert_eq!(DisplayContext::FirstPersonRightHand as i32, 4);
    assert_eq!(DisplayContext::Head as i32, 5);
    assert_eq!(DisplayContext::Gui as i32, 6);
    assert_eq!(DisplayContext::Ground as i32, 7);
    assert_eq!(DisplayContext::Fixed as i32, 8);
    assert_eq!(DisplayContext::default(), DisplayContext::None);
}

#[test]
fn sound_category_ordinals() {
    // vanilla SoundSource / Bukkit SoundCategory ordinals.
    assert_eq!(SoundCategory::Master.wire(), 0);
    assert_eq!(SoundCategory::Music.wire(), 1);
    assert_eq!(SoundCategory::Record.wire(), 2);
    assert_eq!(SoundCategory::Weather.wire(), 3);
    assert_eq!(SoundCategory::Block.wire(), 4);
    assert_eq!(SoundCategory::Hostile.wire(), 5);
    assert_eq!(SoundCategory::Neutral.wire(), 6);
    assert_eq!(SoundCategory::Player.wire(), 7);
    assert_eq!(SoundCategory::Ambient.wire(), 8);
    assert_eq!(SoundCategory::Voice.wire(), 9);
    // Record by default: players who turn music off still hear a billboard.
    assert_eq!(SoundCategory::default(), SoundCategory::Record);
}

#[test]
fn pose_part_and_equipment_slot_ordinals() {
    // ABI: part 0 head, 1 body, 2 l_arm, 3 r_arm, 4 l_leg, 5 r_leg.
    assert_eq!(PosePart::Head as i32, 0);
    assert_eq!(PosePart::Body as i32, 1);
    assert_eq!(PosePart::LeftArm as i32, 2);
    assert_eq!(PosePart::RightArm as i32, 3);
    assert_eq!(PosePart::LeftLeg as i32, 4);
    assert_eq!(PosePart::RightLeg as i32, 5);
    // ALL is in wire order — `raw_apply` relies on it to write a whole pose.
    let wires: Vec<i32> = PosePart::ALL.iter().map(|p| *p as i32).collect();
    assert_eq!(wires, vec![0, 1, 2, 3, 4, 5]);

    // ABI: slot 0 helmet..3 boots, 4 main hand, 5 off hand.
    assert_eq!(EquipmentSlot::Helmet as i32, 0);
    assert_eq!(EquipmentSlot::Chestplate as i32, 1);
    assert_eq!(EquipmentSlot::Leggings as i32, 2);
    assert_eq!(EquipmentSlot::Boots as i32, 3);
    assert_eq!(EquipmentSlot::MainHand as i32, 4);
    assert_eq!(EquipmentSlot::OffHand as i32, 5);
    let wires: Vec<i32> = EquipmentSlot::ALL.iter().map(|s| *s as i32).collect();
    assert_eq!(wires, vec![0, 1, 2, 3, 4, 5]);
}

#[test]
fn text_flag_bits() {
    // bit0 shadow, bit1 see_through, bit2 default_background.
    assert_eq!(TextFlags::default().bits(), 0);
    assert_eq!(
        TextFlags {
            shadow: true,
            ..Default::default()
        }
        .bits(),
        1
    );
    assert_eq!(
        TextFlags {
            see_through: true,
            ..Default::default()
        }
        .bits(),
        2
    );
    assert_eq!(
        TextFlags {
            default_background: true,
            ..Default::default()
        }
        .bits(),
        4
    );
    // All three: 1 + 2 + 4.
    let all = TextFlags {
        shadow: true,
        see_through: true,
        default_background: true,
    };
    assert_eq!(all.bits(), 7);

    // Round trips, and a mixed mask decoded by hand: 5 = shadow + default bg.
    assert_eq!(TextFlags::from_bits(7), all);
    assert_eq!(
        TextFlags::from_bits(5),
        TextFlags {
            shadow: true,
            see_through: false,
            default_background: true,
        }
    );
    // Unknown high bits are the host's business, not ours.
    assert_eq!(TextFlags::from_bits(0b1000_0001), TextFlags::from_bits(1));
}

#[test]
fn stand_flag_bits() {
    // bit0 small, bit1 arms, bit2 no_baseplate, bit3 invisible.
    assert_eq!(StandFlags::default().bits(), 0);
    assert_eq!(
        StandFlags {
            small: true,
            ..Default::default()
        }
        .bits(),
        1
    );
    assert_eq!(
        StandFlags {
            arms: true,
            ..Default::default()
        }
        .bits(),
        2
    );
    assert_eq!(
        StandFlags {
            no_baseplate: true,
            ..Default::default()
        }
        .bits(),
        4
    );
    assert_eq!(
        StandFlags {
            invisible: true,
            ..Default::default()
        }
        .bits(),
        8
    );

    // The usual "floating armour" combination: arms + no base plate +
    // invisible = 2 + 4 + 8 = 14.
    let floating = StandFlags {
        small: false,
        arms: true,
        no_baseplate: true,
        invisible: true,
    };
    assert_eq!(floating.bits(), 14);
    assert_eq!(StandFlags::from_bits(14), floating);
    // 9 = small + invisible.
    assert_eq!(
        StandFlags::from_bits(9),
        StandFlags {
            small: true,
            arms: false,
            no_baseplate: false,
            invisible: true,
        }
    );
}

#[test]
fn argb_packing_round_trips() {
    // Text-display backgrounds cross as 0xAARRGGBB in an i64.
    let c = Color::rgba(0x12, 0x34, 0x56, 0x78);
    assert_eq!(c.to_argb_i64(), 0x7812_3456);
    assert_eq!(Color::from_argb_i64(0x7812_3456), c);
    // Opaque black and fully transparent, by hand.
    assert_eq!(Color::from_argb_i64(0xFF00_0000), Color::BLACK);
    assert_eq!(Color::from_argb_i64(0).a, 0);
}

#[test]
fn item_strings_accept_ids_and_give_syntax() {
    // A registry id converts in, so its spelling is checked at compile time.
    let from_id: ItemStr = items::DIAMOND_SWORD.into();
    assert_eq!(from_id.as_str(), "minecraft:diamond_sword");
    // A raw /give string passes straight through.
    let raw: ItemStr = "minecraft:stone[minecraft:custom_model_data=3]".into();
    assert_eq!(
        raw.as_str(),
        "minecraft:stone[minecraft:custom_model_data=3]"
    );

    // Components accumulate into one bracketed list.
    let head = items::PLAYER_HEAD
        .into_item()
        .with("minecraft:profile", "{name:'Notch'}");
    assert_eq!(
        head.as_str(),
        "minecraft:player_head[minecraft:profile={name:'Notch'}]"
    );
    let two = head.with("minecraft:enchantment_glint_override", "true");
    assert_eq!(
        two.as_str(),
        "minecraft:player_head[minecraft:profile={name:'Notch'},\
         minecraft:enchantment_glint_override=true]"
    );

    // Custom-namespace ids still work — the server validates at use.
    const CUSTOM: ItemId = ItemId::new("mypack:neon_sign");
    assert_eq!(CUSTOM.into_item().as_str(), "mypack:neon_sign");
}

#[test]
fn block_states_accept_registry_ids_by_value_and_reference() {
    // Particle::block and every block setter take `impl Into<BlockState>`.
    assert_eq!(
        BlockState::from(blocks::RED_CONCRETE).as_str(),
        "minecraft:red_concrete"
    );
    assert_eq!(
        BlockState::from(&blocks::RED_CONCRETE).as_str(),
        "minecraft:red_concrete"
    );
    let owned = BlockState::new("minecraft:furnace[lit=true]");
    assert_eq!(BlockState::from(&owned), owned);
}

#[test]
fn wire_values_decode_back_to_their_variant() {
    // The exact decoders the raw ABI layer uses on the way in.
    for (value, mode) in [
        (0, BillboardMode::Fixed),
        (1, BillboardMode::Vertical),
        (2, BillboardMode::Horizontal),
        (3, BillboardMode::Center),
    ] {
        assert_eq!(BillboardMode::from_wire(value), mode);
        assert_eq!(mode.wire(), value);
    }
    for (value, ctx) in [
        (0, DisplayContext::None),
        (2, DisplayContext::ThirdPersonRightHand),
        (5, DisplayContext::Head),
        (6, DisplayContext::Gui),
        (8, DisplayContext::Fixed),
    ] {
        assert_eq!(DisplayContext::from_wire(value), ctx);
        assert_eq!(ctx.wire(), value);
    }
}

#[test]
#[should_panic(expected = "unknown billboard mode")]
fn an_unknown_billboard_mode_kills_rather_than_guessing() {
    // Quietly defaulting would hide a real ABI mismatch.
    let _ = BillboardMode::from_wire(9);
}

#[test]
#[should_panic(expected = "unknown display context")]
fn an_unknown_display_context_kills_rather_than_guessing() {
    let _ = DisplayContext::from_wire(-1);
}
