//! Sound and particles: fire-and-forget effects with no handles.
//!
//! Both are one-shot — you tell the host to play or emit, and that's the end of
//! it. Nothing to own, nothing to despawn, nothing to clean up. Both route
//! through the instance's viewer set, so a `per_player` billboard's sound is
//! heard only by its own viewer.
//!
//! ```ignore
//! sound("minecraft:block.note_block.pling", pos)
//!     .volume(2.0)
//!     .pitch(1.2)
//!     .category(SoundCategory::Record)
//!     .play();
//!
//! particle(Particle::Dust { color: Color::hex("#ff6b35"), size: 1.5 }, pos)
//!     .count(20)
//!     .offset(Offset::splat(0.5))
//!     .speed(0.1)
//!     .emit();
//! ```

mod particle;
mod sound;

#[doc(hidden)]
pub use particle::ParticleWire;
pub use particle::{Particle, ParticleBuilder, particle};
pub use sound::{SoundBuilder, SoundCategory, sound};
