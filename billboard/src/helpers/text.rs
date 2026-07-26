//! Text effects for [`TextDisplay`]: [`typewriter`] and [`marquee`].
//!
//! Both are **blocking**: they sleep between frames, so call them from a task
//! whose job is that sign.
//!
//! # These slice text, so keep markup out of the animated string
//!
//! A text display's content is MiniMessage, and cutting `"<red>hello"` after
//! three characters gives `"<re"` — not red, not "hel", just broken. So these
//! helpers work on the plain string you pass and slice it by **character**
//! (never by byte, so multi-byte characters and emoji stay intact). Style the
//! whole display instead: colour it with one wrapping tag applied to every
//! frame via [`typewriter_styled`], or set the display's own attributes.

use crate::entity::{Dead, TextDisplay, WeakMut};
use crate::math::Ticks;

/// The frames a typewriter plays: growing prefixes of `text`, one per character,
/// from the first character to the whole string.
///
/// Pure and cheap (the frames borrow `text`), and the reason the boundary
/// handling is testable without a server.
pub fn typewriter_frames(text: &str) -> Vec<&str> {
    text.char_indices()
        .map(|(i, c)| &text[..i + c.len_utf8()])
        .collect()
}

/// The frames a marquee plays: a `window`-character view sliding along `text`
/// and wrapping around, one frame per character of `text`.
///
/// If `window` is zero, or at least as long as `text`, there is nothing to
/// scroll and the whole text is the only frame.
pub fn marquee_frames(text: &str, window: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if window == 0 || window >= chars.len() {
        return vec![text.to_owned()];
    }
    (0..chars.len())
        .map(|start| {
            (0..window)
                .map(|i| chars[(start + i) % chars.len()])
                .collect()
        })
        .collect()
}

/// Reveal `text` one character at a time, `per_char` ticks apart. Blocks for
/// `per_char × character count` ticks.
pub fn typewriter(display: &mut TextDisplay, text: &str, per_char: Ticks) {
    for frame in typewriter_frames(text) {
        display.set_text(frame);
        crate::sleep(per_char);
    }
}

/// [`typewriter`], wrapping each frame in `prefix` and `suffix` — the way to
/// keep MiniMessage styling on text that is being sliced:
///
/// ```ignore
/// typewriter_styled(&mut sign, "<bold><gold>", "</gold></bold>", "NOW OPEN", Ticks::new(2));
/// ```
pub fn typewriter_styled(
    display: &mut TextDisplay,
    prefix: &str,
    suffix: &str,
    text: &str,
    per_char: Ticks,
) {
    let mut buf = String::with_capacity(prefix.len() + text.len() + suffix.len());
    for frame in typewriter_frames(text) {
        buf.clear();
        buf.push_str(prefix);
        buf.push_str(frame);
        buf.push_str(suffix);
        display.set_text(&buf);
        crate::sleep(per_char);
    }
}

/// [`typewriter`] through a weak reference. Stops and reports if the display
/// dies mid-sentence.
pub fn typewriter_weak(
    display: &mut WeakMut<TextDisplay>,
    text: &str,
    per_char: Ticks,
) -> Result<(), Dead> {
    for frame in typewriter_frames(text) {
        display.set_text(frame)?;
        crate::sleep(per_char);
    }
    Ok(())
}

/// Scroll `text` through a `window`-character view, `step` ticks per character,
/// for `cycles` full passes. Blocks for the whole run.
pub fn marquee(display: &mut TextDisplay, text: &str, window: usize, step: Ticks, cycles: u32) {
    let frames = marquee_frames(text, window);
    for _ in 0..cycles {
        for frame in &frames {
            display.set_text(frame);
            crate::sleep(step);
        }
    }
}

/// [`marquee`] through a weak reference. Stops and reports if the display dies
/// mid-scroll.
pub fn marquee_weak(
    display: &mut WeakMut<TextDisplay>,
    text: &str,
    window: usize,
    step: Ticks,
    cycles: u32,
) -> Result<(), Dead> {
    let frames = marquee_frames(text, window);
    for _ in 0..cycles {
        for frame in &frames {
            display.set_text(frame)?;
            crate::sleep(step);
        }
    }
    Ok(())
}
