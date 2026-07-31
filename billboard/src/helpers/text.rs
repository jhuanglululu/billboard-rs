//! Text effects for [`TextDisplay`]: [`typewriter`] and [`marquee`], plus
//! [`escape`] and [`styled`] for putting arbitrary strings inside markup.
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

/// Make an arbitrary string safe to drop into MiniMessage.
///
/// # Why you need it
///
/// A text display's content is **MiniMessage source**, parsed by the server
/// with Adventure's default parser — the full grammar: `<red>`, `<bold>`,
/// `<gradient:#a:#b>`, `<rainbow>`, `<hover:…>`, `<click:…>`, `<lang:…>`, the
/// lot, non-strict (an unclosed `<gray>` styles the rest of the line, which is
/// idiomatic, and an unknown tag such as `<notatag>` is passed through as
/// literal text rather than rejected). What the parser *does* reject — a
/// malformed tag, a `<gradient:notacolour>`, an unterminated quote in a tag
/// argument — **kills the animation**, from inside `set_text`. There is no
/// "renders as garbage" outcome to fall back on.
///
/// So the moment you interpolate a string you did not write — a player name, a
/// value read from a payload, anything with a `<` in it — you are either
/// producing markup you did not mean or handing the parser a syntax error that
/// ends the run. `escape` removes both possibilities.
///
/// # The rule
///
/// Two characters carry meaning: `<` opens a tag, and `\` escapes. So `\`
/// becomes `\\` (first, or it would double the backslashes we add) and `<`
/// becomes `\<`. Everything else — `>`, quotes, colons, newlines, emoji — is
/// already literal to the parser and is left alone.
///
/// This is character-for-character the rule the plugin itself applies to
/// untrusted text before showing it to players, so guest and host agree on what
/// "escaped" means.
///
/// ```
/// use billboard::helpers::text;
///
/// assert_eq!(text::escape("<red>"), "\\<red>");
/// assert_eq!(text::escape("plain text"), "plain text");
/// ```
///
/// ```ignore
/// // Style is yours, content is theirs:
/// sign.set_text(format!("<gold>{}</gold> wins", text::escape(player_name)));
/// ```
pub fn escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '<' => out.push_str("\\<"),
            _ => out.push(c),
        }
    }
    out
}

/// Your markup around their text: `prefix + escape(body) + suffix`.
///
/// The one-shot form of what [`typewriter_styled`] does per frame, and the
/// answer to "I need to put a computed value inside a tag". Styling you wrote
/// passes through as markup; `body` is escaped, so a `<` in it is a literal `<`
/// and never a tag, a parse error, or an animation-ending one (see
/// [`escape`]).
///
/// ```
/// use billboard::helpers::text;
///
/// assert_eq!(text::styled("<gold>", "42%", "</gold>"), "<gold>42%</gold>");
/// // The body cannot open a tag of its own, however it was computed:
/// assert_eq!(
///     text::styled("<gray>", "<red>", "</gray>"),
///     "<gray>\\<red></gray>"
/// );
/// ```
///
/// ```ignore
/// // A live readout whose value comes from data and whose colour comes from you:
/// readout.set_text(text::styled("<bold><aqua>", &format!("{steps} steps"), "</aqua></bold>"));
/// ```
///
/// Nothing checks that `prefix` and `suffix` are *valid* MiniMessage or that
/// they balance — they are yours, and a malformed tag of your own still kills
/// the run from inside `set_text`. What this guarantees is that `body` cannot
/// be the thing that breaks them.
pub fn styled(prefix: &str, body: &str, suffix: &str) -> String {
    let escaped = escape(body);
    let mut out = String::with_capacity(prefix.len() + escaped.len() + suffix.len());
    out.push_str(prefix);
    out.push_str(&escaped);
    out.push_str(suffix);
    out
}

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
