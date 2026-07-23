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
}
