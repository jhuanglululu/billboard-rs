//! The shared shape of every entity handle, as macros.
//!
//! Five entity kinds × three handle flavours (owner, [`WeakRef`], [`WeakMut`])
//! × the attribute groups they have in common is a lot of identical code. Each
//! macro here emits one attribute group for one flavour, so an entity module
//! declares which groups it has and then only hand-writes what is genuinely
//! its own.
//!
//! The rules the macros encode, once:
//! - owner methods never return `Result` — misuse kills the animation;
//! - weak methods check liveness first and return `Result<_, Dead>`;
//! - getters always ask the host (no caching, ever);
//! - a plain setter is instant, its `*_to` sibling interpolates over `Ticks`
//!   and returns immediately (non-blocking);
//! - math parameters are `impl AsRef<T>`, so one value serves many entities.
//!
//! Paths inside the expansions are absolute, so an entity module needs no
//! particular imports for them to work.
//!
//! [`WeakRef`]: crate::entity::WeakRef
//! [`WeakMut`]: crate::entity::WeakMut

/// The owner handle itself: the struct, its `Entity`/sealed impls, weak-ref
/// constructors, despawn/leak, and the RAII `Drop`.
///
/// `Drop` despawns unconditionally, and that stays sound now that an owner can
/// be *moved* into a task: the tasks of an instance share one linear memory, so
/// a moved handle is the same one allocation with the same single owner, and it
/// is dropped exactly once — in whichever task ended up holding it. What is
/// still impossible is two tasks owning it at once, which is ordinary Rust
/// ownership rather than anything this macro has to arrange.
macro_rules! entity_handle {
    ($(#[$meta:meta])* $t:ident => $state:ty) => {
        $(#[$meta])*
        #[derive(Debug)]
        pub struct $t {
            id: i32,
        }

        impl crate::entity::sealed::Sealed for $t {}

        impl crate::entity::Entity for $t {
            type State = $state;
        }

        impl $t {
            fn from_id(id: i32) -> $t {
                $t { id }
            }

            /// A read-only weak reference (aliveness + getters).
            pub fn weak(&self) -> crate::entity::WeakRef<$t> {
                crate::entity::WeakRef::from_id(self.id)
            }

            /// A weak reference that can drive the entity but never kill it.
            pub fn weak_mut(&self) -> crate::entity::WeakMut<$t> {
                crate::entity::WeakMut::from_id(self.id)
            }

            /// Despawn now. Equivalent to dropping the handle; reads better at
            /// the end of a scope.
            pub fn despawn(self) {
                // Drop does the work.
            }

            /// Give up ownership: the entity lives until the animation ends
            /// (the host despawns everything then). Returns a
            /// [`WeakMut`](crate::entity::WeakMut) to keep driving it.
            pub fn leak(self) -> crate::entity::WeakMut<$t> {
                let weak = self.weak_mut();
                ::core::mem::forget(self);
                weak
            }
        }

        impl Drop for $t {
            fn drop(&mut self) {
                crate::entity::raw::despawn(self.id);
            }
        }
    };
}

/// Whole-state checkpoints: `state()`, instant `set()`, interpolated
/// `animate()`.
///
/// Expects the enclosing module to define `raw_state(id) -> State` and
/// `raw_apply(id, &State, Ticks)`.
macro_rules! state_api {
    (owner $t:ident, $state:ty) => {
        impl $t {
            /// The entity's current state, freshly queried from the host.
            pub fn state(&self) -> $state {
                raw_state(self.id)
            }

            /// Apply a whole checkpoint instantly.
            pub fn set(&mut self, state: &$state) {
                raw_apply(self.id, state, crate::math::Ticks::new(0));
            }

            /// Apply a whole checkpoint over `over` ticks. Non-blocking:
            /// returns immediately; `sleep` to wait it out. Fields that can't
            /// interpolate switch instantly.
            pub fn animate(&mut self, state: &$state, over: crate::math::Ticks) {
                raw_apply(self.id, state, over);
            }
        }
    };
    (weak $t:ident, $state:ty) => {
        impl crate::entity::WeakRef<$t> {
            /// The entity's current state, freshly queried from the host.
            pub fn state(&self) -> Result<$state, crate::entity::Dead> {
                self.check()?;
                Ok(raw_state(self.id()))
            }
        }

        impl crate::entity::WeakMut<$t> {
            /// The entity's current state, freshly queried from the host.
            pub fn state(&self) -> Result<$state, crate::entity::Dead> {
                self.check()?;
                Ok(raw_state(self.id()))
            }

            /// Apply a whole checkpoint instantly.
            pub fn set(&mut self, state: &$state) -> Result<(), crate::entity::Dead> {
                self.check()?;
                raw_apply(self.id(), state, crate::math::Ticks::new(0));
                Ok(())
            }

            /// Apply a whole checkpoint over `over` ticks (non-blocking).
            pub fn animate(
                &mut self,
                state: &$state,
                over: crate::math::Ticks,
            ) -> Result<(), crate::entity::Dead> {
                self.check()?;
                raw_apply(self.id(), state, over);
                Ok(())
            }
        }
    };
}

/// Position: `position()`, instant `teleport_to()`, interpolated `move_to()`.
/// Every entity kind has these — for armor stands and items the host tweens.
macro_rules! position_api {
    (owner $t:ident) => {
        impl $t {
            /// Current position, freshly queried from the host.
            pub fn position(&self) -> crate::math::Position {
                crate::entity::raw::get_position(self.id)
            }

            /// Teleport instantly.
            pub fn teleport_to(&mut self, position: impl AsRef<crate::math::Position>) {
                crate::entity::raw::set_position(
                    self.id,
                    position.as_ref(),
                    crate::math::Ticks::new(0),
                );
            }

            /// Move to `position` over `over` ticks (non-blocking).
            pub fn move_to(
                &mut self,
                position: impl AsRef<crate::math::Position>,
                over: crate::math::Ticks,
            ) {
                crate::entity::raw::set_position(self.id, position.as_ref(), over);
            }
        }
    };
    (weak $t:ident) => {
        impl crate::entity::WeakRef<$t> {
            pub fn position(&self) -> Result<crate::math::Position, crate::entity::Dead> {
                self.check()?;
                Ok(crate::entity::raw::get_position(self.id()))
            }
        }

        impl crate::entity::WeakMut<$t> {
            pub fn position(&self) -> Result<crate::math::Position, crate::entity::Dead> {
                self.check()?;
                Ok(crate::entity::raw::get_position(self.id()))
            }

            pub fn teleport_to(
                &mut self,
                position: impl AsRef<crate::math::Position>,
            ) -> Result<(), crate::entity::Dead> {
                self.check()?;
                crate::entity::raw::set_position(
                    self.id(),
                    position.as_ref(),
                    crate::math::Ticks::new(0),
                );
                Ok(())
            }

            pub fn move_to(
                &mut self,
                position: impl AsRef<crate::math::Position>,
                over: crate::math::Ticks,
            ) -> Result<(), crate::entity::Dead> {
                self.check()?;
                crate::entity::raw::set_position(self.id(), position.as_ref(), over);
                Ok(())
            }
        }
    };
}

/// Rotation and scale — the display-only half of the transform. Armor stands
/// use yaw instead, and items have neither, so those types simply don't get
/// these methods (a compile error beats a runtime kill).
macro_rules! orientation_api {
    (owner $t:ident) => {
        impl $t {
            /// Current rotation, freshly queried from the host.
            pub fn rotation(&self) -> crate::math::Rotation {
                crate::entity::raw::get_rotation(self.id)
            }

            /// Set the rotation instantly.
            pub fn set_rotation(&mut self, rotation: impl AsRef<crate::math::Rotation>) {
                crate::entity::raw::set_rotation(
                    self.id,
                    rotation.as_ref(),
                    crate::math::Ticks::new(0),
                );
            }

            /// Rotate over `over` ticks (non-blocking).
            pub fn rotate_to(
                &mut self,
                rotation: impl AsRef<crate::math::Rotation>,
                over: crate::math::Ticks,
            ) {
                crate::entity::raw::set_rotation(self.id, rotation.as_ref(), over);
            }

            /// Current scale, freshly queried from the host.
            pub fn scale(&self) -> crate::math::Scale {
                crate::entity::raw::get_scale(self.id)
            }

            /// Set the scale instantly.
            pub fn set_scale(&mut self, scale: impl AsRef<crate::math::Scale>) {
                crate::entity::raw::set_scale(self.id, scale.as_ref(), crate::math::Ticks::new(0));
            }

            /// Rescale over `over` ticks (non-blocking).
            pub fn scale_to(
                &mut self,
                scale: impl AsRef<crate::math::Scale>,
                over: crate::math::Ticks,
            ) {
                crate::entity::raw::set_scale(self.id, scale.as_ref(), over);
            }
        }
    };
    (weak $t:ident) => {
        impl crate::entity::WeakRef<$t> {
            pub fn rotation(&self) -> Result<crate::math::Rotation, crate::entity::Dead> {
                self.check()?;
                Ok(crate::entity::raw::get_rotation(self.id()))
            }

            pub fn scale(&self) -> Result<crate::math::Scale, crate::entity::Dead> {
                self.check()?;
                Ok(crate::entity::raw::get_scale(self.id()))
            }
        }

        impl crate::entity::WeakMut<$t> {
            pub fn rotation(&self) -> Result<crate::math::Rotation, crate::entity::Dead> {
                self.check()?;
                Ok(crate::entity::raw::get_rotation(self.id()))
            }

            pub fn set_rotation(
                &mut self,
                rotation: impl AsRef<crate::math::Rotation>,
            ) -> Result<(), crate::entity::Dead> {
                self.check()?;
                crate::entity::raw::set_rotation(
                    self.id(),
                    rotation.as_ref(),
                    crate::math::Ticks::new(0),
                );
                Ok(())
            }

            pub fn rotate_to(
                &mut self,
                rotation: impl AsRef<crate::math::Rotation>,
                over: crate::math::Ticks,
            ) -> Result<(), crate::entity::Dead> {
                self.check()?;
                crate::entity::raw::set_rotation(self.id(), rotation.as_ref(), over);
                Ok(())
            }

            pub fn scale(&self) -> Result<crate::math::Scale, crate::entity::Dead> {
                self.check()?;
                Ok(crate::entity::raw::get_scale(self.id()))
            }

            pub fn set_scale(
                &mut self,
                scale: impl AsRef<crate::math::Scale>,
            ) -> Result<(), crate::entity::Dead> {
                self.check()?;
                crate::entity::raw::set_scale(
                    self.id(),
                    scale.as_ref(),
                    crate::math::Ticks::new(0),
                );
                Ok(())
            }

            pub fn scale_to(
                &mut self,
                scale: impl AsRef<crate::math::Scale>,
                over: crate::math::Ticks,
            ) -> Result<(), crate::entity::Dead> {
                self.check()?;
                crate::entity::raw::set_scale(self.id(), scale.as_ref(), over);
                Ok(())
            }
        }
    };
}

/// `billboard_mode()` / `set_billboard_mode()` — every display kind has it.
macro_rules! billboard_mode_api {
    (owner $t:ident) => {
        impl $t {
            /// How this display turns to face viewers, freshly queried from
            /// the host.
            pub fn billboard_mode(&self) -> crate::entity::BillboardMode {
                crate::entity::raw::get_billboard_mode(self.id)
            }

            /// Set how this display turns to face viewers.
            pub fn set_billboard_mode(&mut self, mode: crate::entity::BillboardMode) {
                crate::entity::raw::set_billboard_mode(self.id, mode);
            }
        }
    };
    (weak $t:ident) => {
        impl crate::entity::WeakRef<$t> {
            pub fn billboard_mode(
                &self,
            ) -> Result<crate::entity::BillboardMode, crate::entity::Dead> {
                self.check()?;
                Ok(crate::entity::raw::get_billboard_mode(self.id()))
            }
        }

        impl crate::entity::WeakMut<$t> {
            pub fn billboard_mode(
                &self,
            ) -> Result<crate::entity::BillboardMode, crate::entity::Dead> {
                self.check()?;
                Ok(crate::entity::raw::get_billboard_mode(self.id()))
            }

            pub fn set_billboard_mode(
                &mut self,
                mode: crate::entity::BillboardMode,
            ) -> Result<(), crate::entity::Dead> {
                self.check()?;
                crate::entity::raw::set_billboard_mode(self.id(), mode);
                Ok(())
            }
        }
    };
}

/// `item()` / `set_item()` — for the two entity kinds that show an item.
macro_rules! item_api {
    (owner $t:ident) => {
        impl $t {
            /// The item being shown, freshly queried from the host.
            pub fn item(&self) -> crate::entity::ItemStr {
                crate::entity::raw::get_item(self.id)
            }

            /// Swap the item (instant — items can't interpolate). Validated by
            /// the server; anything it rejects kills the animation.
            pub fn set_item(&mut self, item: impl Into<crate::entity::ItemStr>) {
                crate::entity::raw::set_item(self.id, &item.into());
            }
        }
    };
    (weak $t:ident) => {
        impl crate::entity::WeakRef<$t> {
            pub fn item(&self) -> Result<crate::entity::ItemStr, crate::entity::Dead> {
                self.check()?;
                Ok(crate::entity::raw::get_item(self.id()))
            }
        }

        impl crate::entity::WeakMut<$t> {
            pub fn item(&self) -> Result<crate::entity::ItemStr, crate::entity::Dead> {
                self.check()?;
                Ok(crate::entity::raw::get_item(self.id()))
            }

            pub fn set_item(
                &mut self,
                item: impl Into<crate::entity::ItemStr>,
            ) -> Result<(), crate::entity::Dead> {
                self.check()?;
                crate::entity::raw::set_item(self.id(), &item.into());
                Ok(())
            }
        }
    };
}
