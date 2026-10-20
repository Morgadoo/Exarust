//! The player's key bindings as the original stores them: a 23-slot table
//! of virtual-key codes, a saved copy with a version word, a controller-mode
//! flag and the count, and the mapping the game uses to read the keys.
//! Rules: `docs/PLAYER.md` (Bindings).

use bevy::input::ButtonInput;
use bevy::prelude::*;

/// The 23 actions of the original's binding table, in slot
/// order. Slot 0 is the primary mouse button; every later slot is a key or
/// the middle mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Primary attack, drag or use (left mouse button).
    Primary,
    Forward,
    Left,
    Back,
    Right,
    Crouch,
    /// Run and sprint (either shift).
    Run,
    Alternate,
    /// Orbit camera (middle mouse button).
    Orbit,
    /// Cursor Look (V in the shipped table; the settings template shows
    /// SPACE, which is not the shipped slot).
    CursorLook,
    /// Interact (either control).
    Interact,
    Inventory,
    ToggleCombat,
    Skills,
    SwitchEquipped,
    FlipWeapon,
    Journal,
    Talk,
    /// Slots 18–20 hold the power actions; which of use, bar and tree each
    /// one is has not been tied to the settings labels yet.
    UsePower,
    PowerBar,
    /// Use or previous power (Q).
    PowerQuick,
    /// Next power (right arrow).
    PowerNext,
    /// Previous power (left arrow).
    PowerPrevious,
}

impl Action {
    /// The slot in the original's 23-entry table.
    pub const fn slot(self) -> usize {
        self as usize
    }
}

/// The shipped virtual-key codes, in slot order: left mouse, W, A, S, D, X,
/// shift, alt, middle mouse, V, control, I, tab, K, R, F, J, T, C, P, Q,
/// right arrow, left arrow.
pub const DEFAULT_KEYS: [u32; 23] = [
    0x01, 0x57, 0x41, 0x53, 0x44, 0x58, 0x10, 0x12, 0x04, 0x56, 0x11, 0x49, 0x09, 0x4B, 0x52, 0x46,
    0x4A, 0x54, 0x43, 0x50, 0x51, 0x27, 0x25,
];

/// The bindings file's version word.
pub const VERSION: u32 = 101;
/// The table's slot count.
pub const COUNT: usize = 23;

/// A player's bindings: the controller-mode flag and 23 virtual-key codes.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct Bindings {
    /// Whether the controller drives the player instead of the keyboard.
    pub controller: bool,
    keys: [u32; COUNT],
}

impl Default for Bindings {
    fn default() -> Self {
        Self {
            controller: false,
            keys: DEFAULT_KEYS,
        }
    }
}

impl Bindings {
    /// The virtual-key code in `action`'s slot.
    pub fn key(&self, action: Action) -> u32 {
        self.keys[action.slot()]
    }

    /// Binds `action` to `key`.
    pub fn set_key(&mut self, action: Action, key: u32) {
        self.keys[action.slot()] = key;
    }

    /// The table's codes.
    pub fn keys(&self) -> &[u32; COUNT] {
        &self.keys
    }

    /// The Bevy key for `action`, when it is a keyboard key.
    pub fn key_code(&self, action: Action) -> Option<KeyCode> {
        key_code(self.key(action))
    }

    /// The mouse button for `action`, when it is one.
    pub fn mouse_button(&self, action: Action) -> Option<MouseButton> {
        mouse_button(self.key(action))
    }

    /// Whether `action`'s key is held. Shift, alt and control accept either
    /// side, as the original's virtual-key codes do.
    pub fn pressed(&self, keys: &ButtonInput<KeyCode>, action: Action) -> bool {
        let code = self.key(action);
        match code {
            0x10 => keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
            0x11 => keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]),
            0x12 => keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]),
            _ => self.key_code(action).is_some_and(|key| keys.pressed(key)),
        }
    }

    /// Whether `action`'s mouse button is held.
    pub fn mouse_pressed(&self, buttons: &ButtonInput<MouseButton>, action: Action) -> bool {
        self.mouse_button(action)
            .is_some_and(|button| buttons.pressed(button))
    }

    /// Whether `action`'s mouse button went down this frame.
    pub fn mouse_just_pressed(&self, buttons: &ButtonInput<MouseButton>, action: Action) -> bool {
        self.mouse_button(action)
            .is_some_and(|button| buttons.just_pressed(button))
    }

    /// Whether `action`'s key went down this frame.
    pub fn just_pressed(&self, keys: &ButtonInput<KeyCode>, action: Action) -> bool {
        let code = self.key(action);
        match code {
            0x10 => keys.just_pressed(KeyCode::ShiftLeft) || keys.just_pressed(KeyCode::ShiftRight),
            0x11 => {
                keys.just_pressed(KeyCode::ControlLeft) || keys.just_pressed(KeyCode::ControlRight)
            }
            0x12 => keys.just_pressed(KeyCode::AltLeft) || keys.just_pressed(KeyCode::AltRight),
            _ => self
                .key_code(action)
                .is_some_and(|key| keys.just_pressed(key)),
        }
    }

    /// The saved file: version, controller flag, count, then the 23 codes.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(12 + COUNT * 4);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&u32::from(self.controller).to_le_bytes());
        out.extend_from_slice(&(COUNT as u32).to_le_bytes());
        for key in self.keys {
            out.extend_from_slice(&key.to_le_bytes());
        }
        out
    }

    /// Reads a saved file. A file without the controller flag is an older
    /// one: its word after the version is the count.
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let word = |i: usize| -> Option<u32> {
            bytes
                .get(i * 4..i * 4 + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        };
        if word(0)? != VERSION {
            return None;
        }
        let first = word(1)?;
        let (controller, count_at) = if first as usize == COUNT {
            (false, 1)
        } else {
            (first != 0, 2)
        };
        if word(count_at)? as usize != COUNT {
            return None;
        }
        let mut keys = [0; COUNT];
        for (i, key) in keys.iter_mut().enumerate() {
            *key = word(count_at + 1 + i)?;
        }
        Some(Self { controller, keys })
    }
}

/// The Bevy key for a Windows virtual-key code in the table.
pub fn key_code(code: u32) -> Option<KeyCode> {
    Some(match code {
        0x57 => KeyCode::KeyW,
        0x41 => KeyCode::KeyA,
        0x53 => KeyCode::KeyS,
        0x44 => KeyCode::KeyD,
        0x58 => KeyCode::KeyX,
        0x10 => KeyCode::ShiftLeft,
        0x12 => KeyCode::AltLeft,
        0x56 => KeyCode::KeyV,
        0x11 => KeyCode::ControlLeft,
        0x49 => KeyCode::KeyI,
        0x09 => KeyCode::Tab,
        0x4B => KeyCode::KeyK,
        0x52 => KeyCode::KeyR,
        0x46 => KeyCode::KeyF,
        0x4A => KeyCode::KeyJ,
        0x54 => KeyCode::KeyT,
        0x43 => KeyCode::KeyC,
        0x50 => KeyCode::KeyP,
        0x51 => KeyCode::KeyQ,
        0x27 => KeyCode::ArrowRight,
        0x25 => KeyCode::ArrowLeft,
        _ => return None,
    })
}

/// The Bevy mouse button for a Windows virtual-key code in the table.
pub fn mouse_button(code: u32) -> Option<MouseButton> {
    Some(match code {
        0x01 => MouseButton::Left,
        0x02 => MouseButton::Right,
        0x04 => MouseButton::Middle,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_table_is_the_original_23_slots() {
        assert_eq!(DEFAULT_KEYS[Action::Primary.slot()], 0x01);
        assert_eq!(DEFAULT_KEYS[Action::Forward.slot()], 0x57);
        assert_eq!(DEFAULT_KEYS[Action::Left.slot()], 0x41);
        assert_eq!(DEFAULT_KEYS[Action::Back.slot()], 0x53);
        assert_eq!(DEFAULT_KEYS[Action::Right.slot()], 0x44);
        assert_eq!(DEFAULT_KEYS[Action::Crouch.slot()], 0x58);
        assert_eq!(DEFAULT_KEYS[Action::Run.slot()], 0x10);
        assert_eq!(DEFAULT_KEYS[Action::Alternate.slot()], 0x12);
        assert_eq!(DEFAULT_KEYS[Action::Orbit.slot()], 0x04);
        assert_eq!(DEFAULT_KEYS[Action::CursorLook.slot()], 0x56);
        assert_eq!(DEFAULT_KEYS[Action::Interact.slot()], 0x11);
        assert_eq!(DEFAULT_KEYS[Action::ToggleCombat.slot()], 0x09);
        assert_eq!(DEFAULT_KEYS[Action::PowerQuick.slot()], 0x51);
        assert_eq!(DEFAULT_KEYS[Action::PowerNext.slot()], 0x27);
        assert_eq!(DEFAULT_KEYS[Action::PowerPrevious.slot()], 0x25);
        assert_eq!(DEFAULT_KEYS.len(), COUNT);
    }

    #[test]
    fn a_saved_table_round_trips_and_the_old_shape_loads() {
        let bindings = Bindings {
            controller: true,
            ..default()
        };
        let bytes = bindings.encode();
        assert_eq!(Bindings::decode(&bytes), Some(bindings));

        // Older file: version, count, keys; no controller flag.
        let mut old = VERSION.to_le_bytes().to_vec();
        old.extend_from_slice(&(COUNT as u32).to_le_bytes());
        for key in DEFAULT_KEYS {
            old.extend_from_slice(&key.to_le_bytes());
        }
        let decoded = Bindings::decode(&old).expect("the old shape loads");
        assert!(!decoded.controller);
        assert_eq!(decoded.keys, DEFAULT_KEYS);

        // A wrong count or a truncated file is refused.
        let mut wrong = old.clone();
        wrong[4..8].copy_from_slice(&22u32.to_le_bytes());
        assert_eq!(Bindings::decode(&wrong), None);
        assert_eq!(Bindings::decode(&bytes[..bytes.len() - 1]), None);
    }

    #[test]
    fn the_shift_alt_and_control_codes_accept_either_side() {
        let bindings = Bindings::default();
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::ShiftRight);
        keys.press(KeyCode::AltRight);
        keys.press(KeyCode::ControlRight);
        assert!(bindings.pressed(&keys, Action::Run));
        assert!(bindings.pressed(&keys, Action::Alternate));
        assert!(bindings.pressed(&keys, Action::Interact));
        assert!(!bindings.pressed(&keys, Action::Forward));
        assert_eq!(bindings.key_code(Action::CursorLook), Some(KeyCode::KeyV));
        assert_eq!(
            bindings.mouse_button(Action::Primary),
            Some(MouseButton::Left)
        );
        assert_eq!(
            bindings.mouse_button(Action::Orbit),
            Some(MouseButton::Middle)
        );
    }
}
