//! The pieces of the retail engine this mod drives.
//!
//! The follow camera and the level loader are found by byte pattern, never by
//! a fixed address. Layouts below were read from Exanima 0.9.5.2g
//! (`context/re/`); a later build that moves a field will simply stop
//! matching rather than write somewhere else.

use std::ffi::c_void;

use crate::scan::Module;

/// Vertical field of view, degrees, at this offset of the camera node.
const FOV: usize = 0x1C8;
/// Near and far planes, centimetres.
const NEAR: usize = 0x1CC;
const FAR: usize = 0x1D0;
/// 3×4 basis of the camera node: rows are right, up, back, then position.
const BASIS: usize = 0x40;
/// Set after the basis changes, so the renderer rebuilds its matrices.
const DIRTY: usize = 0xE4;
/// The camera node a navigation controller looks through.
const NODE: usize = 0x190;
/// Far enough that leaving a level's bounds does not clip the view.
const FAR_PLANE: f32 = 100_000.0;

type FollowCamera = unsafe extern "C" fn(*mut c_void, f32) -> u64;
type LoadLevel = unsafe extern "C" fn(*mut i8, i32);

static mut ORIGINAL_FOLLOW: Option<FollowCamera> = None;
static mut ORIGINAL_LOAD: Option<LoadLevel> = None;

/// The camera node the game is currently looking through.
static CAMERA: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub fn install(module: &Module) -> Result<(), &'static str> {
    let follow = module
        .find(FOLLOW_PATTERN)
        .ok_or("follow camera not found")?;
    let load = module.find(LOAD_PATTERN).ok_or("level loader not found")?;
    unsafe {
        ORIGINAL_FOLLOW = Some(std::mem::transmute(minhook_create(follow, follow_hook as _)?));
        ORIGINAL_LOAD = Some(std::mem::transmute(minhook_create(load, load_hook as _)?));
        minhook::MinHook::enable_all_hooks().map_err(|_| "hooks would not enable")?;
    }
    Ok(())
}

pub fn remove() {
    unsafe {
        let _ = minhook::MinHook::disable_all_hooks();
    }
}

/// Where the camera is, in centimetres, if one has been seen this session.
pub fn camera_position() -> Option<[f32; 3]> {
    let node = CAMERA.load(std::sync::atomic::Ordering::Relaxed) as *mut u8;
    if node.is_null() {
        return None;
    }
    Some(unsafe { read_vec3(node.add(BASIS + 0x24)) })
}

/// Points the camera and moves it. `forward` is the direction it looks.
pub fn place(position: [f32; 3], forward: [f32; 3], fov: f32) {
    let node = CAMERA.load(std::sync::atomic::Ordering::Relaxed) as *mut u8;
    if node.is_null() {
        return;
    }
    let back = normalize(mul(forward, -1.0));
    let mut right = normalize(cross([0.0, 1.0, 0.0], back));
    if length(right) < 1e-4 {
        right = [1.0, 0.0, 0.0];
    }
    let up = normalize(cross(back, right));
    unsafe {
        write_vec3(node.add(BASIS), right);
        write_vec3(node.add(BASIS + 0x0C), up);
        write_vec3(node.add(BASIS + 0x18), back);
        write_vec3(node.add(BASIS + 0x24), position);
        *(node.add(FOV) as *mut f32) = fov;
        *(node.add(NEAR) as *mut f32) = 5.0;
        *(node.add(FAR) as *mut f32) = FAR_PLANE;
        *(node.add(DIRTY) as *mut u32) = 1;
    }
}

/// Asks the game to load `name` (a level file stem, such as `exanima02`).
pub fn load_level(name: &str) {
    let Some(load) = (unsafe { ORIGINAL_LOAD }) else {
        return;
    };
    let mut bytes = name.as_bytes().to_vec();
    bytes.push(0);
    unsafe { load(bytes.as_mut_ptr().cast(), 0) };
}

/// The game's own follow camera. While free cam is on it only records which
/// camera node is live, and does not move it.
unsafe extern "C" fn follow_hook(controller: *mut c_void, time: f32) -> u64 {
    if !controller.is_null() {
        let node = unsafe { *(controller as *mut u8).add(NODE).cast::<*mut u8>() };
        if !node.is_null() {
            CAMERA.store(node as usize, std::sync::atomic::Ordering::Relaxed);
        }
    }
    if crate::fly::flying() {
        return 0;
    }
    unsafe { ORIGINAL_FOLLOW.unwrap()(controller, time) }
}

unsafe extern "C" fn load_hook(name: *mut i8, flags: i32) {
    unsafe { ORIGINAL_LOAD.unwrap()(name, flags) };
}

unsafe fn minhook_create(
    target: *mut c_void,
    detour: *mut c_void,
) -> Result<*mut c_void, &'static str> {
    unsafe { minhook::MinHook::create_hook(target, detour) }.map_err(|_| "hook was rejected")
}

unsafe fn read_vec3(at: *mut u8) -> [f32; 3] {
    unsafe { [read_f32(at), read_f32(at.add(4)), read_f32(at.add(8))] }
}

unsafe fn write_vec3(at: *mut u8, value: [f32; 3]) {
    for (index, component) in value.into_iter().enumerate() {
        unsafe { (at.add(index * 4) as *mut f32).write(component) };
    }
}

unsafe fn read_f32(at: *mut u8) -> f32 {
    unsafe { (at as *const f32).read() }
}

fn mul(vector: [f32; 3], scale: f32) -> [f32; 3] {
    [vector[0] * scale, vector[1] * scale, vector[2] * scale]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn length(vector: [f32; 3]) -> f32 {
    (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt()
}

fn normalize(vector: [f32; 3]) -> [f32; 3] {
    let length = length(vector);
    if length < 1e-8 {
        [0.0, 0.0, 1.0]
    } else {
        mul(vector, 1.0 / length)
    }
}

/// Opening of the follow camera (`0x100094720` in 0.9.5.2g): it keeps its
/// controller in `rbx` and reads the controller's animator at `+0x80`.
const FOLLOW_PATTERN: &str = "\
    53 48 8D 64 24 A0 66 0F 7F 74 24 40 66 44 0F 7F 44 24 50 \
    48 89 CB 0F 28 F1 48 83 BB 80 00 00 00 00";

/// Opening of the level loader (`0x100181450`): it clears a flag, checks the
/// world object, then calls into the world setup.
const LOAD_PATTERN: &str = "\
    53 56 48 8D 64 24 88 48 89 CB C6 05 ? ? ? ? 00 \
    48 83 3D ? ? ? ? 00 75 05 E8 ? ? ? ? 48 8B 05";
