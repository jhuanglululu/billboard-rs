//! [`Color`], the [`Oklab`] perceptual space it interpolates through, and
//! [`Gradient`].

use bytemuck::{Pod, Zeroable};

/// An 8-bit sRGB colour with alpha. Plain data (`Pod`), so it crosses channels
/// and sits inside other payload structs.
///
/// Feeds dust particles, MiniMessage tags, and text-display backgrounds.
/// Blending goes through [`Oklab`] — see [`lerp`](Color::lerp).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Pod, Zeroable)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    /// Fully transparent.
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);

    /// Opaque colour from 8-bit components.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color { r, g, b, a }
    }

    /// Opaque colour from a packed `0xRRGGBB` literal — the `const` form of
    /// [`hex`](Color::hex), for tables and constants.
    pub const fn rgb_hex(hex: u32) -> Color {
        Color::rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
    }

    /// Parse `"#rrggbb"` or `"#rrggbbaa"` (the leading `#` is optional).
    ///
    /// A string that isn't one of those kills the animation: a colour literal
    /// is written once by a human and either right or a typo, so failing loudly
    /// beats rendering something almost-right.
    pub fn hex(s: &str) -> Color {
        let body = s.strip_prefix('#').unwrap_or(s);
        assert!(
            body.len() == 6 || body.len() == 8,
            "Color::hex expects \"#rrggbb\" or \"#rrggbbaa\", got {s:?}"
        );
        let bytes = body.as_bytes();
        let mut parts = [255u8; 4];
        for (i, pair) in bytes.chunks_exact(2).enumerate() {
            parts[i] = nibble(pair[0], s) << 4 | nibble(pair[1], s);
        }
        Color {
            r: parts[0],
            g: parts[1],
            b: parts[2],
            a: parts[3],
        }
    }

    /// Pack as `0xAARRGGBB` in an `i64` — the wire form of a text display's
    /// background colour.
    pub const fn to_argb_i64(self) -> i64 {
        ((self.a as i64) << 24) | ((self.r as i64) << 16) | ((self.g as i64) << 8) | self.b as i64
    }

    /// Unpack a `0xAARRGGBB` value — the inverse of
    /// [`to_argb_i64`](Color::to_argb_i64), for reading a text display's
    /// background back from the host. Bits above the low 32 are ignored.
    pub const fn from_argb_i64(argb: i64) -> Color {
        Color {
            a: (argb >> 24) as u8,
            r: (argb >> 16) as u8,
            g: (argb >> 8) as u8,
            b: argb as u8,
        }
    }

    /// Convert to Oklab. Alpha is not part of Oklab and is carried separately.
    pub fn to_oklab(self) -> Oklab {
        let (r, g, b) = (
            srgb_to_linear(self.r),
            srgb_to_linear(self.g),
            srgb_to_linear(self.b),
        );
        // Linear sRGB -> LMS-like cone responses, cube-rooted, -> Oklab.
        let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
        let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
        let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;
        let (l, m, s) = (l.cbrt(), m.cbrt(), s.cbrt());
        Oklab {
            l: 0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
            a: 1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
            b: 0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
        }
    }

    /// Convert back from Oklab, keeping this colour's alpha. Components
    /// outside the sRGB gamut are clamped.
    pub fn from_oklab(lab: Oklab, alpha: u8) -> Color {
        let l = lab.l + 0.3963377774 * lab.a + 0.2158037573 * lab.b;
        let m = lab.l - 0.1055613458 * lab.a - 0.0638541728 * lab.b;
        let s = lab.l - 0.0894841775 * lab.a - 1.2914855480 * lab.b;
        let (l, m, s) = (l * l * l, m * m * m, s * s * s);
        Color {
            r: linear_to_srgb(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s),
            g: linear_to_srgb(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s),
            b: linear_to_srgb(-0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s),
            a: alpha,
        }
    }

    /// Blend towards `other` in Oklab, `t` clamped to `[0, 1]`. Perceptual, so
    /// a red→green fade stays bright the whole way instead of sagging through
    /// mud — the same choice CSS `color-mix` makes. Alpha blends linearly.
    ///
    /// `t` clamps (an "overshot" colour has no meaning — the channels would just
    /// saturate), but a *non-finite* `t` kills: it is a bug upstream, not an
    /// extreme.
    pub fn lerp(self, other: Color, t: f64) -> Color {
        assert!(t.is_finite(), "Color::lerp called with a non-finite t: {t}");
        let t = t.clamp(0.0, 1.0);
        let lab = self.to_oklab().lerp(other.to_oklab(), t);
        let a = self.a as f64 + (other.a as f64 - self.a as f64) * t;
        Color::from_oklab(lab, round_u8(a))
    }
}

/// One hex digit, or a clean kill.
fn nibble(c: u8, whole: &str) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => panic!("Color::hex got a non-hex digit in {whole:?}"),
    }
}

/// One sRGB byte to linear light, via the sRGB transfer function.
fn srgb_to_linear(c: u8) -> f64 {
    let c = c as f64 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear light back to an sRGB byte, clamped into gamut.
fn linear_to_srgb(c: f64) -> u8 {
    let c = if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    round_u8(c.clamp(0.0, 1.0) * 255.0)
}

fn round_u8(v: f64) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

/// A colour in Björn Ottosson's **Oklab**: `l` is perceptual lightness
/// (`0`..`1` for in-gamut sRGB), `a` green→red, `b` blue→yellow.
///
/// Euclidean distance in this space is roughly perceptual difference, which is
/// what makes it the right space both for interpolation and for
/// [`BlockPalette`](super::BlockPalette)'s nearest-block search.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct Oklab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

impl Oklab {
    pub const fn new(l: f64, a: f64, b: f64) -> Oklab {
        Oklab { l, a, b }
    }

    /// Straight-line blend, `t` clamped to `[0, 1]`. A non-finite `t` kills.
    pub fn lerp(self, other: Oklab, t: f64) -> Oklab {
        assert!(t.is_finite(), "Oklab::lerp called with a non-finite t: {t}");
        let t = t.clamp(0.0, 1.0);
        Oklab {
            l: self.l + (other.l - self.l) * t,
            a: self.a + (other.a - self.a) * t,
            b: self.b + (other.b - self.b) * t,
        }
    }

    /// Squared perceptual distance — the comparison form, no `sqrt`.
    pub fn distance_squared(self, other: Oklab) -> f64 {
        let (dl, da, db) = (self.l - other.l, self.a - other.a, self.b - other.b);
        dl * dl + da * da + db * db
    }

    /// Perceptual distance.
    pub fn distance(self, other: Oklab) -> f64 {
        self.distance_squared(other).sqrt()
    }
}

/// A colour ramp: stops at positions along `0.0..=1.0`, sampled in Oklab.
///
/// Stops are sorted on construction, so they can be listed in any order.
/// Sampling clamps: below the first stop gives the first colour, above the last
/// gives the last.
///
/// # Cost
///
/// Each stop's Oklab is computed **once, on construction**, because sRGB→Oklab
/// costs three `powf`s and three `cbrt`s and those are software routines in the
/// plugin's interpreter. [`sample`](Gradient::sample) therefore does one blend
/// and one Oklab→sRGB conversion, whatever the stop count — but build the
/// gradient outside your per-tick loop, not inside it.
#[derive(Clone, Debug, PartialEq)]
pub struct Gradient {
    stops: Vec<(f64, Color)>,
    /// `stops[i].1` in Oklab, in the same order — the expensive half of a
    /// sample, hoisted out of the hot path.
    oklab: Vec<Oklab>,
}

impl Gradient {
    /// Build from `(position, colour)` stops. At least one stop is required —
    /// an empty gradient has no colour to give and kills the animation.
    pub fn new(stops: impl IntoIterator<Item = (f64, Color)>) -> Gradient {
        let mut stops: Vec<(f64, Color)> = stops.into_iter().collect();
        assert!(!stops.is_empty(), "Gradient::new needs at least one stop");
        assert!(
            stops.iter().all(|(t, _)| t.is_finite()),
            "Gradient stop positions must be finite"
        );
        stops.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("finite stop positions"));
        Gradient::from_sorted(stops)
    }

    /// The one place a `Gradient` is built, so the cached Oklab can never fall
    /// out of step with the stops.
    fn from_sorted(stops: Vec<(f64, Color)>) -> Gradient {
        let oklab = stops.iter().map(|(_, c)| c.to_oklab()).collect();
        Gradient { stops, oklab }
    }

    /// A two-stop ramp from `from` at `0.0` to `to` at `1.0`.
    pub fn two(from: Color, to: Color) -> Gradient {
        Gradient::from_sorted(vec![(0.0, from), (1.0, to)])
    }

    /// Evenly spaced stops (`0.0` to `1.0`) through the given colours.
    pub fn even(colors: impl IntoIterator<Item = Color>) -> Gradient {
        let colors: Vec<Color> = colors.into_iter().collect();
        assert!(
            !colors.is_empty(),
            "Gradient::even needs at least one colour"
        );
        let last = colors.len().saturating_sub(1) as f64;
        let stops = colors
            .into_iter()
            .enumerate()
            .map(|(i, c)| (if last == 0.0 { 0.0 } else { i as f64 / last }, c))
            .collect();
        Gradient::from_sorted(stops)
    }

    /// The stops, sorted by position.
    pub fn stops(&self) -> &[(f64, Color)] {
        &self.stops
    }

    /// The colour at `t`, blending the two surrounding stops in Oklab.
    ///
    /// Out-of-range `t` clamps to the end stops; a non-finite `t` kills, because
    /// every comparison against NaN is false and the sample would fall through
    /// to whatever the last branch happened to be.
    pub fn sample(&self, t: f64) -> Color {
        assert!(
            t.is_finite(),
            "Gradient::sample called with a non-finite t: {t}"
        );
        let first = self.stops[0];
        let last = self.stops[self.stops.len() - 1];
        if t <= first.0 {
            return first.1;
        }
        if t >= last.0 {
            return last.1;
        }
        // Fewer than a handful of stops in practice; a scan beats a binary
        // search and keeps the code obvious.
        let hi = self
            .stops
            .iter()
            .position(|(pos, _)| *pos >= t)
            .unwrap_or(self.stops.len() - 1);
        let (t0, c0) = self.stops[hi - 1];
        let (t1, c1) = self.stops[hi];
        if t1 == t0 {
            // Coincident stops: a hard edge, take the later colour.
            return c1;
        }
        // Identical to `c0.lerp(c1, f)` — but the endpoints' Oklab is already
        // known, so only the way back out to sRGB is computed here.
        let f = (t - t0) / (t1 - t0);
        let lab = self.oklab[hi - 1].lerp(self.oklab[hi], f);
        let alpha = c0.a as f64 + (c1.a as f64 - c0.a as f64) * f;
        Color::from_oklab(lab, round_u8(alpha))
    }

    /// The colour at `t`, left in [`Oklab`] — no conversion at all.
    ///
    /// For a gradient feeding a [`BlockPalette`](super::BlockPalette), this and
    /// [`nearest_to_oklab`](super::BlockPalette::nearest_to_oklab) skip a round
    /// trip out to sRGB and straight back again.
    pub fn sample_oklab(&self, t: f64) -> Oklab {
        assert!(
            t.is_finite(),
            "Gradient::sample_oklab called with a non-finite t: {t}"
        );
        let last_index = self.stops.len() - 1;
        if t <= self.stops[0].0 {
            return self.oklab[0];
        }
        if t >= self.stops[last_index].0 {
            return self.oklab[last_index];
        }
        let hi = self
            .stops
            .iter()
            .position(|(pos, _)| *pos >= t)
            .unwrap_or(last_index);
        let (t0, t1) = (self.stops[hi - 1].0, self.stops[hi].0);
        if t1 == t0 {
            return self.oklab[hi];
        }
        self.oklab[hi - 1].lerp(self.oklab[hi], (t - t0) / (t1 - t0))
    }
}
