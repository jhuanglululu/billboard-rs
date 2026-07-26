//! Known-answer tests for the colour layer.
//!
//! The Oklab expectations are the published reference values from Björn
//! Ottosson's article (the same matrices the implementation uses, but the
//! numbers here are the article's, not this code's output). The sRGB
//! round-trips through Oklab were worked out separately with the published
//! transfer function and matrices.

use billboard::helpers::{BlockPalette, Color, Gradient, Oklab};
use billboard::registry::blocks;

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn approx_lab(got: Oklab, l: f64, a: f64, b: f64) {
    assert!(
        approx(got.l, l) && approx(got.a, a) && approx(got.b, b),
        "expected Oklab({l}, {a}, {b}), got {got:?}"
    );
}

#[test]
fn constructors_and_packing() {
    assert_eq!(Color::rgb(1, 2, 3), Color::rgba(1, 2, 3, 255));
    assert_eq!(Color::rgb_hex(0x12_34_56), Color::rgb(0x12, 0x34, 0x56));
    assert_eq!(Color::WHITE, Color::rgb(255, 255, 255));
    assert_eq!(Color::TRANSPARENT.a, 0);

    // 0xAARRGGBB, the text-display background wire form.
    assert_eq!(
        Color::rgba(0x12, 0x34, 0x56, 0x78).to_argb_i64(),
        0x7812_3456
    );
    assert_eq!(Color::BLACK.to_argb_i64(), 0xFF00_0000);
    assert_eq!(Color::TRANSPARENT.to_argb_i64(), 0);
}

#[test]
fn hex_parsing() {
    assert_eq!(Color::hex("#ff6b35"), Color::rgba(255, 0x6b, 0x35, 255));
    // The leading # is optional, and case doesn't matter.
    assert_eq!(Color::hex("FF6B35"), Color::rgb(255, 0x6b, 0x35));
    // 8 digits carry alpha.
    assert_eq!(Color::hex("#0a141e80"), Color::rgba(10, 20, 30, 128));
}

#[test]
#[should_panic(expected = "Color::hex")]
fn hex_with_wrong_length_kills() {
    let _ = Color::hex("#abc");
}

#[test]
#[should_panic(expected = "non-hex digit")]
fn hex_with_a_bad_digit_kills() {
    let _ = Color::hex("#gg0000");
}

#[test]
fn oklab_reference_values() {
    // Ottosson's published values for the sRGB primaries.
    approx_lab(Color::WHITE.to_oklab(), 1.0, 0.0, 0.0);
    approx_lab(Color::BLACK.to_oklab(), 0.0, 0.0, 0.0);
    approx_lab(
        Color::rgb(255, 0, 0).to_oklab(),
        0.6279554,
        0.2248631,
        0.1258463,
    );
    approx_lab(
        Color::rgb(0, 255, 0).to_oklab(),
        0.8664396,
        -0.2338876,
        0.1794985,
    );
    approx_lab(
        Color::rgb(0, 0, 255).to_oklab(),
        0.4520137,
        -0.0324570,
        -0.3115281,
    );
    // Mid gray is achromatic (a = b = 0) but *not* L = 0.5: Oklab lightness
    // is perceptual, and 50% sRGB gray reads as ~0.6.
    approx_lab(Color::rgb(128, 128, 128).to_oklab(), 0.5998708, 0.0, 0.0);
}

#[test]
fn oklab_round_trips_through_srgb() {
    for c in [
        Color::rgb(255, 0, 0),
        Color::rgb(18, 52, 86),
        Color::rgb(200, 150, 25),
        Color::WHITE,
        Color::BLACK,
    ] {
        let back = Color::from_oklab(c.to_oklab(), c.a);
        assert_eq!(back, c, "{c:?} did not survive an Oklab round trip");
    }
}

#[test]
fn lerp_endpoints_and_known_midpoints() {
    let red = Color::rgb(255, 0, 0);
    let blue = Color::rgb(0, 0, 255);
    assert_eq!(red.lerp(blue, 0.0), red);
    assert_eq!(red.lerp(blue, 1.0), blue);
    // Out-of-range t clamps rather than extrapolating.
    assert_eq!(red.lerp(blue, -3.0), red);
    assert_eq!(red.lerp(blue, 7.0), blue);

    // Oklab midpoint of black and white: L = 0.5 is linear 0.125, which the
    // sRGB transfer function encodes as 0.3885 -> 99. (A naive RGB midpoint
    // would be 128 — this is the perceptual difference, visible.)
    assert_eq!(Color::BLACK.lerp(Color::WHITE, 0.5), Color::rgb(99, 99, 99));

    // Red -> blue midpoint: Oklab (0.539985, 0.096203, -0.092841) back to
    // sRGB. Note it stays a bright violet instead of dropping to (128,0,128).
    assert_eq!(red.lerp(blue, 0.5), Color::rgb(140, 83, 162));

    // Alpha interpolates linearly.
    let a = Color::rgba(0, 0, 0, 0);
    let b = Color::rgba(0, 0, 0, 200);
    assert_eq!(a.lerp(b, 0.5).a, 100);
}

#[test]
fn oklab_distance() {
    let a = Oklab::new(0.5, 0.0, 0.0);
    let b = Oklab::new(0.5, 0.3, 0.4);
    // 3-4-5 triangle in the (a, b) plane, scaled by 0.1.
    assert!(approx(a.distance(b), 0.5));
    assert!(approx(a.distance_squared(b), 0.25));
    assert!(approx(a.distance(a), 0.0));
}

#[test]
fn gradient_sampling() {
    let g = Gradient::two(Color::BLACK, Color::WHITE);
    assert_eq!(g.sample(0.0), Color::BLACK);
    assert_eq!(g.sample(1.0), Color::WHITE);
    assert_eq!(g.sample(0.5), Color::rgb(99, 99, 99));
    // Clamped outside 0..1.
    assert_eq!(g.sample(-1.0), Color::BLACK);
    assert_eq!(g.sample(2.0), Color::WHITE);

    // Stops may be given out of order; sampling uses the sorted positions.
    let red = Color::rgb(255, 0, 0);
    let blue = Color::rgb(0, 0, 255);
    let g = Gradient::new([(1.0, blue), (0.0, Color::BLACK), (0.5, red)]);
    assert_eq!(g.stops()[0], (0.0, Color::BLACK));
    assert_eq!(g.sample(0.5), red);
    // Halfway through the *second* segment is the red->blue midpoint.
    assert_eq!(g.sample(0.75), Color::rgb(140, 83, 162));
    // A quarter of the way through the first segment: black -> red at t=0.5
    // of that segment is not sampled here; 0.25 is exactly the segment
    // midpoint, so it equals black.lerp(red, 0.5).
    assert_eq!(g.sample(0.25), Color::BLACK.lerp(red, 0.5));

    // Evenly spaced stops.
    let g = Gradient::even([Color::BLACK, red, blue]);
    assert_eq!(g.stops().len(), 3);
    assert!(approx(g.stops()[1].0, 0.5));
    assert_eq!(g.sample(0.5), red);

    // A single stop is a constant colour.
    let g = Gradient::new([(0.3, red)]);
    assert_eq!(g.sample(0.0), red);
    assert_eq!(g.sample(1.0), red);
}

#[test]
#[should_panic(expected = "at least one stop")]
fn empty_gradient_kills() {
    let _ = Gradient::new([]);
}

#[test]
fn palette_matches_its_own_entries_exactly() {
    // The strongest property: every tabulated colour must resolve to its own
    // block, in all four families (16 entries each).
    for palette in [
        BlockPalette::CONCRETE,
        BlockPalette::WOOL,
        BlockPalette::TERRACOTTA,
        BlockPalette::GLASS,
    ] {
        assert_eq!(palette.entries().len(), 16);
        for entry in palette.entries() {
            assert_eq!(
                palette.nearest(entry.color),
                entry.block,
                "{} did not match its own palette colour",
                entry.block
            );
            // And the stored Oklab is the one matching actually uses.
            assert_eq!(
                palette.nearest_to_oklab(entry.oklab).block,
                entry.block,
                "{} did not match its own stored Oklab",
                entry.block
            );
        }
    }
}

#[test]
fn palette_nearest_for_unambiguous_colours() {
    let concrete = BlockPalette::CONCRETE;
    assert_eq!(concrete.nearest(Color::WHITE), blocks::WHITE_CONCRETE);
    assert_eq!(concrete.nearest(Color::BLACK), blocks::BLACK_CONCRETE);
    assert_eq!(
        concrete.nearest(Color::rgb(0, 0, 255)),
        blocks::BLUE_CONCRETE
    );
    assert_eq!(
        concrete.nearest(Color::rgb(255, 255, 0)),
        blocks::YELLOW_CONCRETE
    );
    // Pure sRGB green is far brighter than green concrete; lime is the
    // perceptually closest vanilla block.
    assert_eq!(
        concrete.nearest(Color::rgb(0, 255, 0)),
        blocks::LIME_CONCRETE
    );

    // A nudge off a table entry still lands on it.
    let nudged = Color::rgb(0x2C + 3, 0x2E - 3, 0x8F + 2);
    assert_eq!(concrete.nearest(nudged), blocks::BLUE_CONCRETE);

    assert_eq!(BlockPalette::WOOL.nearest(Color::WHITE), blocks::WHITE_WOOL);
    assert_eq!(
        BlockPalette::TERRACOTTA.nearest(Color::rgb(0x25, 0x16, 0x10)),
        blocks::BLACK_TERRACOTTA
    );
    assert_eq!(
        BlockPalette::GLASS.nearest(Color::rgb(0x16, 0x9C, 0x9C)),
        blocks::CYAN_STAINED_GLASS
    );
}

/// Nearest-by-Oklab means nearest *perceptually*, lightness included — and no
/// vanilla block is as bright as a fully saturated sRGB primary. Fully
/// saturated red therefore lands on orange concrete (distance 0.103), not red
/// concrete (0.229), which is much darker. Recorded here because it surprises
/// people, and because a future weighting change should have to argue with a
/// test.
#[test]
fn saturated_red_prefers_orange_over_the_much_darker_red() {
    assert_eq!(
        BlockPalette::CONCRETE.nearest(Color::rgb(255, 0, 0)),
        blocks::ORANGE_CONCRETE
    );
    // The real red concrete colour, of course, still matches itself.
    assert_eq!(
        BlockPalette::CONCRETE.nearest(Color::rgb_hex(0x8E2121)),
        blocks::RED_CONCRETE
    );
}

#[test]
fn user_palettes_and_ties() {
    let duotone = [
        (Color::rgb_hex(0x080A0F), blocks::BLACK_CONCRETE),
        (Color::rgb_hex(0xCFD5D6), blocks::WHITE_CONCRETE),
    ];
    let p = BlockPalette::new(&duotone);
    assert_eq!(p.nearest(Color::rgb(20, 20, 20)), blocks::BLACK_CONCRETE);
    assert_eq!(p.nearest(Color::rgb(230, 230, 230)), blocks::WHITE_CONCRETE);

    // A tie resolves to the earlier entry, so results are stable.
    let tied = [
        (Color::rgb(10, 10, 10), blocks::BLACK_CONCRETE),
        (Color::rgb(10, 10, 10), blocks::GRAY_CONCRETE),
    ];
    assert_eq!(
        BlockPalette::new(&tied).nearest(Color::rgb(200, 200, 200)),
        blocks::BLACK_CONCRETE
    );

    // nearest_with reports which palette colour won.
    let (matched, block) = p.nearest_with(Color::rgb(20, 20, 20));
    assert_eq!(matched, Color::rgb_hex(0x080A0F));
    assert_eq!(block, blocks::BLACK_CONCRETE);

    // The precomputed Oklab table lines up with the entries.
    let table = p.oklab_table();
    assert_eq!(table.len(), 2);
    assert_eq!(table[0], Color::rgb_hex(0x080A0F).to_oklab());
}

#[test]
#[should_panic(expected = "empty palette")]
fn empty_palette_kills() {
    let _ = BlockPalette::new(&[]).nearest(Color::WHITE);
}

#[test]
#[should_panic(expected = "non-finite")]
fn a_non_finite_gradient_sample_kills() {
    // Silently returning opaque black would hide a NaN that came from
    // somewhere upstream (a divide by a zero duration, most likely).
    let _ = Gradient::two(Color::BLACK, Color::WHITE).sample(f64::NAN);
}

#[test]
#[should_panic(expected = "non-finite")]
fn an_infinite_gradient_sample_kills() {
    let _ = Gradient::two(Color::BLACK, Color::WHITE).sample(f64::INFINITY);
}

#[test]
#[should_panic(expected = "non-finite")]
fn a_non_finite_colour_blend_kills() {
    let _ = Color::BLACK.lerp(Color::WHITE, f64::NAN);
}

/// The cached stop Oklab must produce exactly what converting on the fly did —
/// this is the invariant that made hoisting the conversion out of `sample` safe.
#[test]
fn sampling_in_oklab_agrees_with_sampling_in_srgb() {
    let red = Color::rgb(255, 0, 0);
    let blue = Color::rgb(0, 0, 255);
    let g = Gradient::new([(0.0, Color::BLACK), (0.5, red), (1.0, blue)]);

    for i in 0..=20 {
        let t = i as f64 / 20.0;
        // `sample` is `sample_oklab` plus the trip back out to sRGB, so for
        // opaque stops the two must agree exactly.
        assert_eq!(
            g.sample(t),
            Color::from_oklab(g.sample_oklab(t), 255),
            "disagreement at t = {t}"
        );
    }

    // Endpoints and clamping behave the same in both.
    assert_eq!(g.sample_oklab(-1.0), Color::BLACK.to_oklab());
    assert_eq!(g.sample_oklab(2.0), blue.to_oklab());
}

#[test]
#[should_panic(expected = "non-finite")]
fn a_non_finite_oklab_sample_kills() {
    let _ = Gradient::two(Color::BLACK, Color::WHITE).sample_oklab(f64::NAN);
}

/// Matching straight from Oklab must pick the same block as matching from the
/// sRGB colour it came from — the palette's fast path for gradient pipelines.
#[test]
fn matching_from_oklab_agrees_with_matching_from_srgb() {
    let ramp = Gradient::new([
        (0.0, Color::hex("#2c2e8f")),
        (0.45, Color::hex("#e06100")),
        (1.0, Color::hex("#f1af15")),
    ]);
    for i in 0..=32 {
        let t = i as f64 / 32.0;
        let colour = ramp.sample(t);
        assert_eq!(
            BlockPalette::CONCRETE.nearest(colour),
            BlockPalette::CONCRETE
                .nearest_to_oklab(colour.to_oklab())
                .block,
            "disagreement at t = {t}"
        );
    }
}
