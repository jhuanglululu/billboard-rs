//! [`sound`]: play a sound at a position.

use crate::abi::marshal;
use crate::math::Position;

/// Which volume slider a sound obeys.
///
/// Wire values are vanilla's `SoundSource` / Bukkit `SoundCategory` ordinals:
/// `0 Master`, `1 Music`, `2 Record`, `3 Weather`, `4 Block`, `5 Hostile`,
/// `6 Neutral`, `7 Player`, `8 Ambient`, `9 Voice`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SoundCategory {
    Master = 0,
    Music = 1,
    /// Jukeboxes and note blocks — the usual choice for a billboard's own
    /// soundtrack, since players who mute music still hear it.
    #[default]
    Record = 2,
    Weather = 3,
    Block = 4,
    Hostile = 5,
    Neutral = 6,
    Player = 7,
    Ambient = 8,
    Voice = 9,
}

impl SoundCategory {
    /// The ABI wire value.
    pub const fn wire(self) -> i32 {
        self as i32
    }
}

/// Start building a sound to play at `position`.
///
/// # Sound ids are never validated
///
/// This is the SDK's **one deliberate exception** to "every error kills the
/// animation". Sound resolution happens on the *client*, and resource packs
/// add ids the server has never heard of, so there is nothing the server could
/// honestly validate against. A typo'd id is silently nothing — no error, no
/// kill. Check your spelling.
pub fn sound(id: impl Into<String>, position: impl AsRef<Position>) -> SoundBuilder {
    SoundBuilder {
        id: id.into(),
        position: *position.as_ref(),
        category: SoundCategory::default(),
        volume: 1.0,
        pitch: 1.0,
    }
}

/// A sound waiting to be [`play`](SoundBuilder::play)ed. Defaults: volume
/// `1.0`, pitch `1.0`, category [`SoundCategory::Record`].
#[derive(Clone, Debug, PartialEq)]
#[must_use = "a sound builder does nothing until you call .play()"]
pub struct SoundBuilder {
    id: String,
    position: Position,
    category: SoundCategory,
    volume: f64,
    pitch: f64,
}

impl SoundBuilder {
    /// Volume. Above `1.0` this widens the audible radius rather than making
    /// the sound louder — vanilla behaviour.
    pub fn volume(mut self, volume: f64) -> SoundBuilder {
        self.volume = volume;
        self
    }

    /// Playback pitch; `0.5`..`2.0` is the range clients honour.
    pub fn pitch(mut self, pitch: f64) -> SoundBuilder {
        self.pitch = pitch;
        self
    }

    pub fn category(mut self, category: SoundCategory) -> SoundBuilder {
        self.category = category;
        self
    }

    /// Play it, for everyone currently viewing this animation.
    ///
    /// There is no stop: a stop-sound packet filters by id and category, so
    /// stopping *this* sound would also silence every other sound sharing them
    /// — including a player's own music.
    pub fn play(self) {
        marshal::play_sound(
            &self.id,
            self.position.x,
            self.position.y,
            self.position.z,
            self.category.wire(),
            self.volume,
            self.pitch,
        );
    }
}
