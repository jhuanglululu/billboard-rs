//! Known-answer tests for the fire-and-forget effects: which host call each
//! [`Particle`] dispatches to, with which converted arguments, and the text
//! effects' frame sequences (the part of a typewriter/marquee that can be wrong
//! without a server).

use billboard::effects::{Particle, ParticleWire};
use billboard::entity::{BlockState, ItemStr};
use billboard::helpers::{Color, text};
use billboard::registry::{blocks, items};

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-12
}

#[test]
fn dust_dispatches_with_channels_normalised_to_unit_range() {
    // emit_particle_dust takes r/g/b as 0.0..=1.0 f64s, so 255 -> 1.0,
    // 0 -> 0.0, 128 -> 128/255 = 0.501960784313…
    let p = Particle::Dust {
        color: Color::rgb(255, 0, 128),
        size: 1.5,
    };
    match p.wire() {
        ParticleWire::Dust { rgb, size } => {
            assert!(approx(rgb.0, 1.0), "red was {}", rgb.0);
            assert!(approx(rgb.1, 0.0), "green was {}", rgb.1);
            assert!(approx(rgb.2, 128.0 / 255.0), "blue was {}", rgb.2);
            assert!(approx(size, 1.5));
        }
        other => panic!("dust must dispatch to emit_particle_dust, got {other:?}"),
    }

    // Alpha is dropped — a dust particle has no transparency to carry.
    let opaque = Particle::Dust {
        color: Color::rgba(10, 20, 30, 255),
        size: 1.0,
    };
    let translucent = Particle::Dust {
        color: Color::rgba(10, 20, 30, 7),
        size: 1.0,
    };
    assert_eq!(opaque.wire(), translucent.wire());
}

#[test]
fn dust_transition_carries_both_colours_in_order() {
    let p = Particle::DustTransition {
        from: Color::rgb(255, 255, 255),
        to: Color::rgb(0, 0, 0),
        size: 2.0,
    };
    assert_eq!(
        p.wire(),
        ParticleWire::DustTransition {
            from: (1.0, 1.0, 1.0),
            to: (0.0, 0.0, 0.0),
            size: 2.0,
        }
    );
}

#[test]
fn block_and_item_particles_dispatch_with_their_strings() {
    // Registry consts convert in, so ids are compile-time checked.
    assert_eq!(
        Particle::block(blocks::RED_CONCRETE).wire(),
        ParticleWire::Block("minecraft:red_concrete")
    );
    assert_eq!(
        Particle::block(blocks::OAK_LOG.state().axis(billboard::registry::Axis::Z)).wire(),
        ParticleWire::Block("minecraft:oak_log[axis=z]")
    );
    assert_eq!(
        Particle::item(items::DIAMOND).wire(),
        ParticleWire::Item("minecraft:diamond")
    );
    // And raw strings still work for anything the registry doesn't have.
    assert_eq!(
        Particle::Block(BlockState::new("mypack:neon")).wire(),
        ParticleWire::Block("mypack:neon")
    );
    assert_eq!(
        Particle::Item(ItemStr::new(
            "minecraft:stone[minecraft:custom_model_data=1]"
        ))
        .wire(),
        ParticleWire::Item("minecraft:stone[minecraft:custom_model_data=1]")
    );
}

#[test]
fn named_particles_dispatch_to_the_generic_call() {
    assert_eq!(
        Particle::named("minecraft:end_rod").wire(),
        ParticleWire::Named("minecraft:end_rod")
    );
    assert_eq!(
        Particle::Named("minecraft:flame".to_owned()).wire(),
        ParticleWire::Named("minecraft:flame")
    );
}

#[test]
fn typewriter_frames_grow_one_character_at_a_time() {
    assert_eq!(text::typewriter_frames("abc"), vec!["a", "ab", "abc"]);
    // An empty string has nothing to type.
    assert!(text::typewriter_frames("").is_empty());
}

#[test]
fn typewriter_frames_respect_utf8_boundaries() {
    // "é" is two bytes, "🎉" is four: slicing by byte index would panic or
    // produce invalid UTF-8, so the frames must land on character boundaries.
    let frames = text::typewriter_frames("héllo 🎉");
    assert_eq!(
        frames,
        vec!["h", "hé", "hél", "héll", "héllo", "héllo ", "héllo 🎉"]
    );
    // One frame per *character*, not per byte (the string is 11 bytes long).
    assert_eq!(frames.len(), 7);
    assert_eq!("héllo 🎉".len(), 11);
    // Every frame is a valid prefix of the whole text.
    for frame in &frames {
        assert!("héllo 🎉".starts_with(frame));
    }
}

#[test]
fn marquee_frames_slide_and_wrap() {
    // Six characters through a three-wide window: six frames, wrapping round.
    assert_eq!(
        text::marquee_frames("abcdef", 3),
        vec!["abc", "bcd", "cde", "def", "efa", "fab"]
    );
}

#[test]
fn marquee_frames_wrap_by_character_not_byte() {
    assert_eq!(text::marquee_frames("aé🎉", 2), vec!["aé", "é🎉", "🎉a"]);
}

#[test]
fn a_window_wider_than_the_text_has_nothing_to_scroll() {
    assert_eq!(text::marquee_frames("ab", 5), vec!["ab"]);
    assert_eq!(text::marquee_frames("ab", 2), vec!["ab"]);
    // A zero window is the same "nothing to do" case rather than an empty
    // frame that would blank the sign.
    assert_eq!(text::marquee_frames("ab", 0), vec!["ab"]);
    assert_eq!(text::marquee_frames("", 3), vec![""]);
}

// --- text::escape, against the host's own rule ---
//
// The plugin escapes untrusted text with `raw.replace("\\", "\\\\")
// .replace("<", "\\<")` and its unit tests pin exactly these cases; the SDK's
// copy has to agree character for character, or a name that is safe on one side
// is markup (or a parse error, which kills) on the other. Expectations below are
// written out by hand, not derived from the implementation.

#[test]
fn escape_neutralises_every_tag_opener() {
    assert_eq!(text::escape("<red>"), "\\<red>");
    // The dangerous one: a click tag would otherwise run a command on click.
    assert_eq!(
        text::escape("<click:run_command:'/op'>"),
        "\\<click:run_command:'/op'>"
    );
    // Only '<' opens a tag, so '>' is ordinary text and stays put.
    assert_eq!(text::escape("a > b"), "a > b");
}

#[test]
fn escape_doubles_backslashes_so_they_cannot_re_enable_a_tag() {
    // A lone backslash is doubled…
    assert_eq!(text::escape("hi\\there"), "hi\\\\there");
    // …and doing so before escaping '<' is what stops "\<red>" (an already
    // escaped tag in the input) from collapsing back into a live tag: the
    // backslash becomes two, and the '<' gains its own.
    assert_eq!(text::escape("\\<red>"), "\\\\\\<red>");
}

#[test]
fn escape_leaves_everything_else_alone() {
    assert_eq!(text::escape("plain text"), "plain text");
    assert_eq!(text::escape(""), "");
    // Colons, quotes, hashes and newlines are only meaningful *inside* a tag.
    assert_eq!(text::escape("a:b '#c'\nd"), "a:b '#c'\nd");
    // Multi-byte characters pass through untouched.
    assert_eq!(text::escape("héllo 🎉"), "héllo 🎉");
}

#[test]
fn escape_handles_every_occurrence_not_just_the_first() {
    assert_eq!(text::escape("<a><b>"), "\\<a>\\<b>");
    assert_eq!(text::escape("\\\\"), "\\\\\\\\");
    assert_eq!(text::escape("<\\<"), "\\<\\\\\\<");
}

// --- text::styled: your markup outside, their text escaped inside ---

#[test]
fn styled_wraps_the_body_without_touching_the_markup() {
    // The ordinary case: prefix and suffix pass through verbatim, body is
    // already safe and comes out unchanged.
    assert_eq!(
        text::styled("<gold>", "42 steps", "</gold>"),
        "<gold>42 steps</gold>"
    );
    // Empty affixes are just escape.
    assert_eq!(text::styled("", "<red>", ""), text::escape("<red>"));
    assert_eq!(text::styled("", "", ""), "");
}

#[test]
fn styled_escapes_only_the_body() {
    // A body that looks like markup stays text — this is the whole point.
    assert_eq!(
        text::styled("<gray>", "<red>danger", "</gray>"),
        "<gray>\\<red>danger</gray>"
    );
    // Backslashes in the body are doubled, exactly as `escape` does…
    assert_eq!(text::styled("<b>", "a\\b", "</b>"), "<b>a\\\\b</b>");
    // …while a backslash the *caller* wrote in the affixes is theirs to mean:
    // an intentionally escaped tag in the prefix survives untouched.
    assert_eq!(text::styled("\\<not_a_tag>", "x", ""), "\\<not_a_tag>x");
}

#[test]
fn styled_is_the_one_shot_form_of_typewriter_styleds_last_frame() {
    // typewriter_styled builds prefix + frame + suffix, and its final frame is
    // the whole string; styled is that, with the body escaped. For a body with
    // no metacharacters the two agree character for character.
    let (prefix, body, suffix) = ("<bold><gold>", "NOW OPEN", "</gold></bold>");
    let last_frame = *text::typewriter_frames(body).last().unwrap();
    assert_eq!(last_frame, body);
    assert_eq!(
        text::styled(prefix, body, suffix),
        format!("{prefix}{body}{suffix}")
    );
}

#[test]
fn builders_are_inert_until_played_or_emitted() {
    // Building must not touch the ABI (the host stubs would panic) — the whole
    // point of the fire-and-forget builders being `#[must_use]`.
    let s = billboard::effects::sound(
        "minecraft:block.note_block.pling",
        billboard::math::Position::ZERO,
    )
    .volume(2.0)
    .pitch(1.2)
    .category(billboard::effects::SoundCategory::Record);
    let p = billboard::effects::particle(
        Particle::Dust {
            color: Color::WHITE,
            size: 1.0,
        },
        billboard::math::Position::ZERO,
    )
    .count(20)
    .offset(billboard::math::Offset::splat(0.5))
    .speed(0.1);
    // Nothing has happened yet; dropping them is silent.
    drop(s);
    drop(p);
}
