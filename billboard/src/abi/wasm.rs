//! The real guest ABI: host functions imported from module `"billboard"`.
//! Compiled only for `wasm32`; see the parent module for the contract.

#[link(wasm_import_module = "billboard")]
unsafe extern "C" {
    pub fn realloc(ptr: *mut u8, old_size: usize, align: usize, new_size: usize) -> *mut u8;
    pub fn fork() -> i32;
    pub fn join(task: i32);
    pub fn kill(task: i32);
    pub fn exit() -> !;
    pub fn sleep(ticks: i64);
    pub fn spawn_block_display(
        state_ptr: *const u8,
        state_len: usize,
        x: f64,
        y: f64,
        z: f64,
    ) -> i32;
    pub fn set_position(entity: i32, x: f64, y: f64, z: f64, over_ticks: i64);
    pub fn set_rotation(entity: i32, qx: f64, qy: f64, qz: f64, qw: f64, over_ticks: i64);
    pub fn set_scale(entity: i32, sx: f64, sy: f64, sz: f64, over_ticks: i64);
    pub fn set_block(entity: i32, state_ptr: *const u8, state_len: usize);
    pub fn get_position(entity: i32, out: *mut f64);
    pub fn get_rotation(entity: i32, out: *mut f64);
    pub fn get_scale(entity: i32, out: *mut f64);
    pub fn get_block_len(entity: i32) -> i32;
    pub fn get_block(entity: i32, buf: *mut u8);
    pub fn despawn(entity: i32);
    pub fn is_alive(entity: i32) -> i32;
    pub fn log(ptr: *const u8, len: usize);
    pub fn fail(ptr: *const u8, len: usize) -> !;

    // --- ABI v2: sync primitives. One host-side id space covers signals,
    // barriers, composites and channels; a wrong-kind op kills. Ids are plain
    // integers in the copied memory, so they survive fork for free. ---
    pub fn signal_new() -> i32;
    pub fn signal_notify(id: i32, mode: i32);
    pub fn barrier_new(n: i32) -> i32;
    pub fn wait_all(a: i32, b: i32) -> i32;
    pub fn wait_any(a: i32, b: i32) -> i32;
    pub fn wait(id: i32);
    pub fn channel_new(cap: i32) -> i32;
    pub fn channel_send(id: i32, ptr: *const u8, len: usize);
    pub fn channel_recv_len(id: i32) -> i32;
    pub fn channel_recv(id: i32, buf: *mut u8);
    pub fn channel_peek_len(id: i32) -> i32;
    pub fn channel_peek(id: i32, buf: *mut u8);
    pub fn channel_try_len(id: i32) -> i32;
    pub fn channel_clear(id: i32);

    // --- ABI v2: randomness. Two host streams (non-deterministic, and the
    // per-instance deterministic one) plus its reseed. ---
    pub fn random_nondet() -> i64;
    pub fn random_det() -> i64;
    pub fn seed_random(seed: i64);

    // --- ABI v2: new entity kinds. The v1 transform imports
    // (`set_position`/`set_rotation`/`set_scale` and their getters) apply to
    // every entity id; these add what is specific to each kind. Entities
    // without client-side interpolation (armor stands, items) take the same
    // `over_ticks` arguments and are tweened host-side. ---
    pub fn spawn_item_display(item_ptr: *const u8, item_len: usize, x: f64, y: f64, z: f64) -> i32;
    pub fn spawn_text_display(text_ptr: *const u8, text_len: usize, x: f64, y: f64, z: f64) -> i32;
    pub fn spawn_armor_stand(x: f64, y: f64, z: f64) -> i32;
    pub fn spawn_item(item_ptr: *const u8, item_len: usize, x: f64, y: f64, z: f64) -> i32;
    pub fn set_item(entity: i32, ptr: *const u8, len: usize);
    pub fn get_item_len(entity: i32) -> i32;
    pub fn get_item(entity: i32, buf: *mut u8);
    pub fn set_display_context(entity: i32, ctx: i32);
    pub fn get_display_context(entity: i32) -> i32;
    pub fn set_billboard_mode(entity: i32, mode: i32);
    pub fn get_billboard_mode(entity: i32) -> i32;
    pub fn set_text(entity: i32, ptr: *const u8, len: usize);
    pub fn get_text_len(entity: i32) -> i32;
    pub fn get_text(entity: i32, buf: *mut u8);
    pub fn set_text_background(entity: i32, argb: i64);
    pub fn get_text_background(entity: i32) -> i64;
    pub fn set_text_opacity(entity: i32, opacity: i64);
    pub fn get_text_opacity(entity: i32) -> i64;
    pub fn set_line_width(entity: i32, width: i64);
    pub fn get_line_width(entity: i32) -> i64;
    pub fn set_text_flags(entity: i32, flags: i32);
    pub fn get_text_flags(entity: i32) -> i32;
    pub fn set_pose(entity: i32, part: i32, x_deg: f64, y_deg: f64, z_deg: f64, over_ticks: i64);
    pub fn get_pose(entity: i32, part: i32, out: *mut f64);
    pub fn set_equipment(entity: i32, slot: i32, ptr: *const u8, len: usize);
    pub fn set_stand_flags(entity: i32, flags: i32);
    pub fn get_stand_flags(entity: i32) -> i32;
    pub fn set_yaw(entity: i32, yaw_deg: f64, over_ticks: i64);
    pub fn get_yaw(entity: i32) -> f64;

    // --- ABI v2: sound & particles. Fire-and-forget, no handles, routed to
    // the instance's viewer set. Sound ids are never validated — the one
    // deliberate exception to the error philosophy. ---
    pub fn play_sound(
        ptr: *const u8,
        len: usize,
        x: f64,
        y: f64,
        z: f64,
        category: i32,
        volume: f64,
        pitch: f64,
    );
    #[allow(clippy::too_many_arguments)]
    pub fn emit_particle(
        name_ptr: *const u8,
        name_len: usize,
        x: f64,
        y: f64,
        z: f64,
        count: i32,
        ox: f64,
        oy: f64,
        oz: f64,
        speed: f64,
    );
    #[allow(clippy::too_many_arguments)]
    pub fn emit_particle_dust(
        r: f64,
        g: f64,
        b: f64,
        size: f64,
        x: f64,
        y: f64,
        z: f64,
        count: i32,
        ox: f64,
        oy: f64,
        oz: f64,
        speed: f64,
    );
    #[allow(clippy::too_many_arguments)]
    pub fn emit_particle_dust_transition(
        fr: f64,
        fg: f64,
        fb: f64,
        tr: f64,
        tg: f64,
        tb: f64,
        size: f64,
        x: f64,
        y: f64,
        z: f64,
        count: i32,
        ox: f64,
        oy: f64,
        oz: f64,
        speed: f64,
    );
    #[allow(clippy::too_many_arguments)]
    pub fn emit_particle_block(
        ptr: *const u8,
        len: usize,
        x: f64,
        y: f64,
        z: f64,
        count: i32,
        ox: f64,
        oy: f64,
        oz: f64,
        speed: f64,
    );
    #[allow(clippy::too_many_arguments)]
    pub fn emit_particle_item(
        ptr: *const u8,
        len: usize,
        x: f64,
        y: f64,
        z: f64,
        count: i32,
        ox: f64,
        oy: f64,
        oz: f64,
        speed: f64,
    );
}
