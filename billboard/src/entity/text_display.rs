//! [`TextDisplay`]: floating text, styled with MiniMessage.

use super::{BillboardMode, Dead, TextFlags, WeakMut, WeakRef, raw};
use crate::helpers::Color;
use crate::math::{Position, Rotation, Scale, Ticks};

/// A [`TextDisplay`]'s complete visible state — a plain-data checkpoint.
#[derive(Clone, Debug, PartialEq)]
pub struct TextDisplayState {
    /// The MiniMessage source string, e.g.
    /// `"<gradient:#ff0000:#0000ff>NOW SHOWING</gradient>"`. Parsed host-side;
    /// invalid markup kills the animation.
    pub text: String,
    /// Background colour, alpha included (`Color::TRANSPARENT` for none).
    pub background: Color,
    /// Text opacity, `0..=255`.
    pub opacity: u8,
    /// Wrap width, in pixels (vanilla's default is 200).
    pub line_width: u32,
    pub flags: TextFlags,
    pub position: Position,
    pub rotation: Rotation,
    pub scale: Scale,
    pub billboard_mode: BillboardMode,
}

fn raw_apply(id: i32, s: &TextDisplayState, over: Ticks) {
    raw::set_position(id, &s.position, over);
    raw::set_rotation(id, &s.rotation, over);
    raw::set_scale(id, &s.scale, over);
    raw::set_text(id, &s.text);
    raw::set_text_background(id, s.background);
    raw::set_text_opacity(id, s.opacity);
    raw::set_line_width(id, s.line_width);
    raw::set_text_flags(id, s.flags);
    raw::set_billboard_mode(id, s.billboard_mode);
}

fn raw_state(id: i32) -> TextDisplayState {
    TextDisplayState {
        text: raw::get_text(id),
        background: raw::get_text_background(id),
        opacity: raw::get_text_opacity(id),
        line_width: raw::get_line_width(id),
        flags: raw::get_text_flags(id),
        position: raw::get_position(id),
        rotation: raw::get_rotation(id),
        scale: raw::get_scale(id),
        billboard_mode: raw::get_billboard_mode(id),
    }
}

entity_handle! {
    /// The absolute owner of a client-side text display entity.
    ///
    /// Text is a **MiniMessage** string — `<red>`, `<bold>`,
    /// `<gradient:…>`, `<rainbow>` and friends — parsed by the server, so
    /// invalid markup kills the animation rather than showing tag soup.
    ///
    /// ```ignore
    /// let mut sign = TextDisplay::spawn("<bold><gold>OPEN", pos);
    /// sign.set_background(Color::rgba(0, 0, 0, 128));
    /// sign.set_billboard_mode(BillboardMode::Center);   // always face viewers
    /// ```
    TextDisplay => TextDisplayState
}

state_api!(owner TextDisplay, TextDisplayState);
state_api!(weak TextDisplay, TextDisplayState);
position_api!(owner TextDisplay);
position_api!(weak TextDisplay);
orientation_api!(owner TextDisplay);
orientation_api!(weak TextDisplay);
billboard_mode_api!(owner TextDisplay);
billboard_mode_api!(weak TextDisplay);

impl TextDisplay {
    /// Spawn a text display at `position` showing `text` (MiniMessage).
    pub fn spawn(text: impl AsRef<str>, position: impl AsRef<Position>) -> TextDisplay {
        TextDisplay::from_id(raw::spawn_text_display(text.as_ref(), position.as_ref()))
    }

    /// The MiniMessage source currently set, freshly queried from the host.
    pub fn text(&self) -> String {
        raw::get_text(self.id)
    }

    /// Replace the text (instant). Invalid MiniMessage kills the animation.
    pub fn set_text(&mut self, text: impl AsRef<str>) {
        raw::set_text(self.id, text.as_ref());
    }

    /// Background colour, alpha included, freshly queried from the host.
    pub fn background(&self) -> Color {
        raw::get_text_background(self.id)
    }

    /// Set the background colour (ARGB); alpha 0 hides it.
    pub fn set_background(&mut self, color: Color) {
        raw::set_text_background(self.id, color);
    }

    /// Text opacity `0..=255`, freshly queried from the host.
    pub fn opacity(&self) -> u8 {
        raw::get_text_opacity(self.id)
    }

    pub fn set_opacity(&mut self, opacity: u8) {
        raw::set_text_opacity(self.id, opacity);
    }

    /// Wrap width in pixels, freshly queried from the host.
    pub fn line_width(&self) -> u32 {
        raw::get_line_width(self.id)
    }

    pub fn set_line_width(&mut self, width: u32) {
        raw::set_line_width(self.id, width);
    }

    /// All three boolean options at once, freshly queried from the host.
    pub fn flags(&self) -> TextFlags {
        raw::get_text_flags(self.id)
    }

    /// Set all three boolean options at once.
    pub fn set_flags(&mut self, flags: TextFlags) {
        raw::set_text_flags(self.id, flags);
    }

    /// Draw a drop shadow behind the text.
    ///
    /// The flags share one ABI bitmask and the SDK caches nothing, so this
    /// reads the current mask from the host, changes one bit, and writes it
    /// back. Setting several at once with [`set_flags`](TextDisplay::set_flags)
    /// is one round trip instead of three.
    pub fn set_shadow(&mut self, shadow: bool) {
        let mut flags = self.flags();
        flags.shadow = shadow;
        self.set_flags(flags);
    }

    /// Render through blocks.
    pub fn set_see_through(&mut self, see_through: bool) {
        let mut flags = self.flags();
        flags.see_through = see_through;
        self.set_flags(flags);
    }

    /// Use the client's default background instead of this display's colour.
    pub fn set_default_background(&mut self, default_background: bool) {
        let mut flags = self.flags();
        flags.default_background = default_background;
        self.set_flags(flags);
    }
}

impl WeakRef<TextDisplay> {
    pub fn text(&self) -> Result<String, Dead> {
        self.check()?;
        Ok(raw::get_text(self.id()))
    }

    pub fn background(&self) -> Result<Color, Dead> {
        self.check()?;
        Ok(raw::get_text_background(self.id()))
    }

    pub fn opacity(&self) -> Result<u8, Dead> {
        self.check()?;
        Ok(raw::get_text_opacity(self.id()))
    }

    pub fn line_width(&self) -> Result<u32, Dead> {
        self.check()?;
        Ok(raw::get_line_width(self.id()))
    }

    pub fn flags(&self) -> Result<TextFlags, Dead> {
        self.check()?;
        Ok(raw::get_text_flags(self.id()))
    }
}

impl WeakMut<TextDisplay> {
    pub fn text(&self) -> Result<String, Dead> {
        self.check()?;
        Ok(raw::get_text(self.id()))
    }

    pub fn set_text(&mut self, text: impl AsRef<str>) -> Result<(), Dead> {
        self.check()?;
        raw::set_text(self.id(), text.as_ref());
        Ok(())
    }

    pub fn background(&self) -> Result<Color, Dead> {
        self.check()?;
        Ok(raw::get_text_background(self.id()))
    }

    pub fn set_background(&mut self, color: Color) -> Result<(), Dead> {
        self.check()?;
        raw::set_text_background(self.id(), color);
        Ok(())
    }

    pub fn opacity(&self) -> Result<u8, Dead> {
        self.check()?;
        Ok(raw::get_text_opacity(self.id()))
    }

    pub fn set_opacity(&mut self, opacity: u8) -> Result<(), Dead> {
        self.check()?;
        raw::set_text_opacity(self.id(), opacity);
        Ok(())
    }

    pub fn line_width(&self) -> Result<u32, Dead> {
        self.check()?;
        Ok(raw::get_line_width(self.id()))
    }

    pub fn set_line_width(&mut self, width: u32) -> Result<(), Dead> {
        self.check()?;
        raw::set_line_width(self.id(), width);
        Ok(())
    }

    pub fn flags(&self) -> Result<TextFlags, Dead> {
        self.check()?;
        Ok(raw::get_text_flags(self.id()))
    }

    pub fn set_flags(&mut self, flags: TextFlags) -> Result<(), Dead> {
        self.check()?;
        raw::set_text_flags(self.id(), flags);
        Ok(())
    }
}
