//! Free flight. The camera is not tied to the player or to a level, so it can
//! leave the playable area.

use std::sync::atomic::{AtomicBool, Ordering};

static FLYING: AtomicBool = AtomicBool::new(false);

pub fn flying() -> bool {
    FLYING.load(Ordering::Relaxed)
}

pub fn toggle() {
    FLYING.fetch_xor(true, Ordering::Relaxed);
}

pub struct Flight {
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub speed: f32,
    pub fov: f32,
    placed: bool,
}

impl Flight {
    pub fn new() -> Self {
        Self {
            position: [0.0, 200.0, 0.0],
            yaw: 0.0,
            pitch: 0.0,
            speed: 1500.0,
            fov: 60.0,
            placed: false,
        }
    }

    /// Adopts the game camera the first time flight is switched on.
    pub fn engage(&mut self) {
        if self.placed {
            return;
        }
        if let Some(position) = crate::game::camera_position() {
            self.position = position;
            self.placed = true;
        }
    }

    pub fn forward(&self) -> [f32; 3] {
        let (yaw, pitch) = (self.yaw, self.pitch);
        [
            yaw.sin() * pitch.cos(),
            pitch.sin(),
            yaw.cos() * pitch.cos(),
        ]
    }

    pub fn look(&mut self, dx: f32, dy: f32) {
        self.yaw += dx;
        self.pitch = (self.pitch + dy).clamp(-1.5, 1.5);
    }

    pub fn move_by(&mut self, along: f32, side: f32, up: f32, seconds: f32) {
        let forward = self.forward();
        let right = [self.yaw.cos(), 0.0, -self.yaw.sin()];
        let step = self.speed * seconds;
        for axis in 0..3 {
            self.position[axis] += (forward[axis] * along + right[axis] * side) * step;
        }
        self.position[1] += up * step;
    }

    pub fn apply(&self) {
        crate::game::place(self.position, self.forward(), self.fov);
    }
}
