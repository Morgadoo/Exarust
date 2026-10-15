//! Keyboard and mouse while free cam is on. Keys are read directly, so the
//! game's own bindings keep working when free cam is off.

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState,     VK_A, VK_CONTROL, VK_D, VK_F1, VK_Q, VK_SHIFT, VK_SPACE, VK_W,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};

pub struct Pointer {
    x: i32,
    y: i32,
}

impl Pointer {
    pub fn at_cursor() -> Self {
        let mut point = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
        unsafe { GetCursorPos(&mut point) };
        Self { x: point.x, y: point.y }
    }

    /// Movement since the cursor was captured, then puts it back, so looking
    /// does not walk the pointer off the window.
    pub fn drag(&mut self) -> (f32, f32) {
        let now = Self::at_cursor();
        let delta = ((now.x - self.x) as f32, (now.y - self.y) as f32);
        unsafe { SetCursorPos(self.x, self.y) };
        delta
    }
}

pub fn pressed(key: Key) -> bool {
    unsafe { GetAsyncKeyState(key as i32) as u16 & 0x8000 != 0 }
}

pub fn just_pressed(key: Key, was: &mut bool) -> bool {
    let down = pressed(key);
    let edge = down && !*was;
    *was = down;
    edge
}

#[derive(Clone, Copy)]
#[repr(i32)]
pub enum Key {
    F1 = VK_F1 as i32,
    W = VK_W as i32,
    A = VK_A as i32,
    S = 0x53,
    D = VK_D as i32,
    Q = VK_Q as i32,
    Space = VK_SPACE as i32,
    Shift = VK_SHIFT as i32,
    Control = VK_CONTROL as i32,
}
