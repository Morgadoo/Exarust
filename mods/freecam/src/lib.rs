//! Free camera for the retail game.
//!
//! Loaded as a DLL (EMTK plugin, or any loader that calls `enable`). F1 shows
//! the panel. Free cam detaches the view from the player, so it can leave the
//! level; the panel loads any other map in the installation.

mod fly;
mod game;
mod input;
mod maps;
mod panel;
mod scan;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Once, mpsc};
use std::time::Instant;

use input::{Key, Pointer};
use panel::Request;

static STARTED: Once = Once::new();
static STOP: AtomicBool = AtomicBool::new(false);

/// Called by the loader once the DLL is fully loaded. Starting threads or
/// windows from `DllMain` runs under the loader lock and kills the game.
#[unsafe(no_mangle)]
pub extern "C" fn enable() {
    STARTED.call_once(|| {
        std::thread::spawn(run);
    });
    STOP.store(false, Ordering::Relaxed);
}

#[unsafe(no_mangle)]
pub extern "C" fn disable() {
    STOP.store(true, Ordering::Relaxed);
    game::remove();
}

fn run() {
    let Some(module) = scan::Module::current() else {
        return;
    };
    if game::install(&module).is_err() {
        return;
    }
    let (sender, receiver) = mpsc::channel();
    panel::open(sender, maps::levels());
    // The panel starts hidden; show it once its window exists.
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(1500));
        panel::show(true);
    });
    let mut flight = fly::Flight::new();
    let mut pointer = Pointer::at_cursor();
    let mut panel_key = false;
    let mut look_key = false;
    let mut last = Instant::now();
    while !STOP.load(Ordering::Relaxed) {
        while let Ok(request) = receiver.try_recv() {
            apply(&mut flight, request);
        }
        if input::just_pressed(Key::F1, &mut panel_key) {
            panel::show(!fly::flying());
            fly::toggle();
        }
        let now = Instant::now();
        let seconds = now.duration_since(last).as_secs_f32().min(0.1);
        last = now;
        if fly::flying() {
            flight.engage();
            steer(&mut flight, &mut pointer, &mut look_key, seconds);
            flight.apply();
        }
        std::thread::sleep(std::time::Duration::from_millis(8));
    }
}

fn apply(flight: &mut fly::Flight, request: Request) {
    match request {
        Request::Toggle => {
            fly::toggle();
            panel::show(fly::flying());
        }
        Request::Load(name) => game::load_level(&name),
        Request::Speed(speed) => flight.speed = speed.clamp(10.0, 100_000.0),
        Request::Fov(fov) => flight.fov = fov.clamp(10.0, 140.0),
    }
}

fn steer(flight: &mut fly::Flight, pointer: &mut Pointer, look_key: &mut bool, seconds: f32) {
    if input::pressed(Key::Control) {
        if !*look_key {
            *pointer = Pointer::at_cursor();
        }
        let (dx, dy) = pointer.drag();
        flight.look(-dx * 0.004, -dy * 0.004);
        *look_key = true;
    } else {
        *look_key = false;
    }
    let along = axis(Key::W, Key::S);
    let side = axis(Key::D, Key::A);
    let up = axis(Key::Space, Key::Q);
    let boost = if input::pressed(Key::Shift) { 4.0 } else { 1.0 };
    let saved = flight.speed;
    flight.speed *= boost;
    flight.move_by(along, side, up, seconds);
    flight.speed = saved;
}

fn axis(positive: Key, negative: Key) -> f32 {
    let mut value = 0.0;
    if input::pressed(positive) {
        value += 1.0;
    }
    if input::pressed(negative) {
        value -= 1.0;
    }
    value
}
