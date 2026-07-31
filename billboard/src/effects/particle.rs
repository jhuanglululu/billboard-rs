//! [`particle`]: emit particles at a position.
//!
//! The three shared knobs — [`count`](ParticleBuilder::count),
//! [`offset`](ParticleBuilder::offset) and [`speed`](ParticleBuilder::speed) —
//! are vanilla's own, forwarded to the particle packet untouched by the plugin,
//! so they mean exactly what `/particle` means by them. Each is documented on
//! its builder method, including the `count = 0` mode that turns `offset` into
//! a direction.
//!
//! Emissions are sent with the protocol's *long distance* flag set, so viewers
//! see them from further away than the vanilla 32-block cutoff — a billboard is
//! usually looked at from across a plaza.

use crate::abi::marshal::{self, ParticleArgs};
use crate::entity::{BlockState, ItemStr};
use crate::helpers::Color;
use crate::math::{Offset, Position};

/// What kind of particle to emit.
///
/// The typed variants cover the ones with parameters worth having typed;
/// [`Named`](Particle::Named) is the escape hatch for the hundreds of
/// parameterless ones (`"minecraft:end_rod"`, `"minecraft:flame"`, …), whose id
/// the server validates.
#[derive(Clone, Debug, PartialEq)]
pub enum Particle {
    /// A coloured dust mote. `size` is its scale (vanilla's default is `1.0`).
    Dust { color: Color, size: f64 },
    /// A dust mote that fades from one colour to another over its lifetime.
    DustTransition { from: Color, to: Color, size: f64 },
    /// A block-breaking fleck, textured with that block.
    Block(BlockState),
    /// An item-breaking fleck, textured with that item.
    Item(ItemStr),
    /// Any other particle, by id.
    Named(String),
}

impl Particle {
    /// A block fleck from anything block-state-like (including
    /// [`blocks`](crate::registry::blocks) consts).
    pub fn block(block: impl Into<BlockState>) -> Particle {
        Particle::Block(block.into())
    }

    /// An item fleck from anything item-like (including
    /// [`items`](crate::registry::items) consts).
    pub fn item(item: impl Into<ItemStr>) -> Particle {
        Particle::Item(item.into())
    }

    /// A particle by id: `Particle::named("minecraft:end_rod")`.
    pub fn named(id: impl Into<String>) -> Particle {
        Particle::Named(id.into())
    }

    /// Which host call this particle maps to, with its arguments already
    /// converted to the ABI's types.
    ///
    /// Public only so the SDK's own tests can check the mapping without a host;
    /// animations have no reason to look at it.
    #[doc(hidden)]
    pub fn wire(&self) -> ParticleWire<'_> {
        match self {
            Particle::Dust { color, size } => ParticleWire::Dust {
                rgb: rgb_f64(*color),
                size: *size,
            },
            Particle::DustTransition { from, to, size } => ParticleWire::DustTransition {
                from: rgb_f64(*from),
                to: rgb_f64(*to),
                size: *size,
            },
            Particle::Block(block) => ParticleWire::Block(block.as_str()),
            Particle::Item(item) => ParticleWire::Item(item.as_str()),
            Particle::Named(id) => ParticleWire::Named(id),
        }
    }
}

/// The host call a [`Particle`] dispatches to. See [`Particle::wire`].
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParticleWire<'a> {
    Named(&'a str),
    Dust {
        rgb: (f64, f64, f64),
        size: f64,
    },
    DustTransition {
        from: (f64, f64, f64),
        to: (f64, f64, f64),
        size: f64,
    },
    Block(&'a str),
    Item(&'a str),
}

/// Colour channels as `0.0..=1.0`, the form the particle imports take. Alpha is
/// dropped — a dust particle has no transparency.
fn rgb_f64(c: Color) -> (f64, f64, f64) {
    (c.r as f64 / 255.0, c.g as f64 / 255.0, c.b as f64 / 255.0)
}

/// Start building a particle emission at `position`.
pub fn particle(particle: Particle, position: impl AsRef<Position>) -> ParticleBuilder {
    ParticleBuilder {
        particle,
        position: *position.as_ref(),
        count: 1,
        offset: Offset::ZERO,
        speed: 0.0,
    }
}

/// A particle emission waiting for [`emit`](ParticleBuilder::emit). Defaults:
/// one particle, no spread, no speed.
#[derive(Clone, Debug, PartialEq)]
#[must_use = "a particle builder does nothing until you call .emit()"]
pub struct ParticleBuilder {
    particle: Particle,
    position: Position,
    count: i32,
    offset: Offset,
    speed: f64,
}

impl ParticleBuilder {
    /// How many particles to spawn — and which of vanilla's two modes the
    /// emission uses.
    ///
    /// `count >= 1` is the normal *cloud* mode: that many particles, each
    /// scattered by [`offset`](ParticleBuilder::offset) and given a random
    /// velocity scaled by [`speed`](ParticleBuilder::speed).
    ///
    /// `count == 0` switches vanilla into *directional* mode: exactly one
    /// particle is spawned at `position` and the offset triple stops being a
    /// spread — it becomes a **velocity vector**, multiplied by `speed`. That is
    /// how you aim a single mote (`.count(0).offset(Offset::new(0.0, 1.0,
    /// 0.0)).speed(0.3)` shoots it upwards). Worth knowing before you compute a
    /// count from data that can reach zero.
    pub fn count(mut self, count: u32) -> ParticleBuilder {
        self.count = i32::try_from(count).expect("particle count overflows i32");
        self
    }

    /// Per-axis spread around `position`, in blocks: vanilla samples a gaussian
    /// with this as its **standard deviation** on each axis, so it is a soft
    /// cloud, not a box — roughly two thirds of the particles land within one
    /// offset of the centre and a few go noticeably further. `Offset::splat(0.2)`
    /// is a tight puff; `1.0` fills a block-sized region loosely.
    ///
    /// When [`count`](ParticleBuilder::count) is `0` this is a velocity vector
    /// instead. See that method.
    pub fn offset(mut self, offset: impl AsRef<Offset>) -> ParticleBuilder {
        self.offset = *offset.as_ref();
        self
    }

    /// How fast the particles set off, in blocks per tick, as a scale on the
    /// random velocity vanilla gives each one: `0.0` leaves them hanging where
    /// they spawned (what you want for a static shape), small values like `0.05`
    /// drift, `1.0` sprays.
    ///
    /// A handful of particle types reinterpret this field entirely — vanilla's
    /// own quirk, e.g. `note` uses it to pick the note's colour — so treat it as
    /// "the protocol's max-speed field" for those.
    ///
    /// With [`count`](ParticleBuilder::count) `0`, it is the multiplier on the
    /// offset-as-velocity vector.
    pub fn speed(mut self, speed: f64) -> ParticleBuilder {
        self.speed = speed;
        self
    }

    /// Emit them, for everyone currently viewing this animation.
    pub fn emit(self) {
        let args = ParticleArgs {
            x: self.position.x,
            y: self.position.y,
            z: self.position.z,
            count: self.count,
            ox: self.offset.x,
            oy: self.offset.y,
            oz: self.offset.z,
            speed: self.speed,
        };
        match self.particle.wire() {
            ParticleWire::Named(id) => marshal::emit_particle(id, args),
            ParticleWire::Dust { rgb, size } => marshal::emit_particle_dust(rgb, size, args),
            ParticleWire::DustTransition { from, to, size } => {
                marshal::emit_particle_dust_transition(from, to, size, args)
            }
            ParticleWire::Block(state) => marshal::emit_particle_block(state, args),
            ParticleWire::Item(item) => marshal::emit_particle_item(item, args),
        }
    }
}
