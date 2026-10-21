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
    /// Opens the power bar (C; the key handler opens it when the player
    /// or the companion has powers).
    PowerBar,
    /// Opens the power tree (P).
    PowerTree,
    /// Uses the picked power (Q).
    UsePower,
    /// Next power (right arrow).
    PowerNext,
    /// Previous power (left arrow).
    PowerPrevious,
}

impl Action {
    /// Every action, in slot order.
    pub const ALL: [Action; 23] = [
        Action::Primary,
        Action::Forward,
        Action::Left,
        Action::Back,
        Action::Right,
        Action::Crouch,
        Action::Run,
        Action::Alternate,
        Action::Orbit,
        Action::CursorLook,
        Action::Interact,
        Action::Inventory,
        Action::ToggleCombat,
        Action::Skills,
        Action::SwitchEquipped,
        Action::FlipWeapon,
        Action::Journal,
        Action::Talk,
        Action::PowerBar,
        Action::PowerTree,
        Action::UsePower,
        Action::PowerNext,
        Action::PowerPrevious,
    ];

    /// The slot in the original's 23-entry table.
    pub const fn slot(self) -> usize {
        self as usize
    }

    /// The action in `slot`.
    pub fn from_slot(slot: usize) -> Option<Self> {
        Self::ALL.get(slot).copied()
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

    /// The name the Controls page shows for `action`'s key. The shipped
    /// table's mouse codes are its own (1, 2 and 4); every other code is
    /// named by the original's rule.
    pub fn key_name(&self, action: Action) -> String {
        match self.key(action) {
            0x01 => "Left mouse".into(),
            0x02 => "Right mouse".into(),
            0x04 => "Middle mouse".into(),
            code => exa_formats::hw_config::key_name(code),
        }
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

    /// Every key of the shipped table comes back to its code.
    #[test]
    fn a_key_names_its_virtual_key() {
        for code in DEFAULT_KEYS {
            if let Some(key) = key_code(code) {
                assert_eq!(virtual_key(key), Some(code), "{key:?}");
            }
        }
        assert_eq!(virtual_key(KeyCode::Digit0), Some(0x30));
        assert_eq!(virtual_key(KeyCode::Digit9), Some(0x39));
        assert_eq!(virtual_key(KeyCode::KeyZ), Some(0x5A));
        assert_eq!(virtual_key(KeyCode::ControlRight), Some(0x11));
        assert_eq!(virtual_key(KeyCode::F1), Some(0x70));
        assert_eq!(virtual_key(KeyCode::Fn), None);
    }

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
        assert_eq!(DEFAULT_KEYS[Action::PowerBar.slot()], 0x43);
        assert_eq!(DEFAULT_KEYS[Action::PowerTree.slot()], 0x50);
        assert_eq!(DEFAULT_KEYS[Action::UsePower.slot()], 0x51);
        assert_eq!(DEFAULT_KEYS[Action::PowerNext.slot()], 0x27);
        assert_eq!(DEFAULT_KEYS[Action::PowerPrevious.slot()], 0x25);
        assert_eq!(DEFAULT_KEYS.len(), COUNT);
        for (slot, action) in Action::ALL.iter().enumerate() {
            assert_eq!(action.slot(), slot);
            assert_eq!(Action::from_slot(slot), Some(*action));
        }
        assert_eq!(Action::from_slot(COUNT), None);
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

/// The Windows virtual-key code for a Bevy key, for the rebinding screen.
/// Keys without a code here are ignored: `None`.
pub fn virtual_key(code: KeyCode) -> Option<u32> {
    Some(match code {
        KeyCode::Backspace => 0x08,
        KeyCode::Tab => 0x09,
        KeyCode::Enter => 0x0D,
        KeyCode::ShiftLeft => 0x10,
        KeyCode::ShiftRight => 0x10,
        KeyCode::ControlLeft => 0x11,
        KeyCode::ControlRight => 0x11,
        KeyCode::AltLeft => 0x12,
        KeyCode::AltRight => 0x12,
        KeyCode::CapsLock => 0x14,
        KeyCode::Escape => 0x1B,
        KeyCode::Space => 0x20,
        KeyCode::PageUp => 0x21,
        KeyCode::PageDown => 0x22,
        KeyCode::End => 0x23,
        KeyCode::Home => 0x24,
        KeyCode::ArrowLeft => 0x25,
        KeyCode::ArrowUp => 0x26,
        KeyCode::ArrowRight => 0x27,
        KeyCode::ArrowDown => 0x28,
        KeyCode::Insert => 0x2D,
        KeyCode::Delete => 0x2E,
        KeyCode::Digit0 => 0x30,
        KeyCode::Digit1 => 0x31,
        KeyCode::Digit2 => 0x32,
        KeyCode::Digit3 => 0x33,
        KeyCode::Digit4 => 0x34,
        KeyCode::Digit5 => 0x35,
        KeyCode::Digit6 => 0x36,
        KeyCode::Digit7 => 0x37,
        KeyCode::Digit8 => 0x38,
        KeyCode::Digit9 => 0x39,
        KeyCode::KeyA => 0x41,
        KeyCode::KeyB => 0x42,
        KeyCode::KeyC => 0x43,
        KeyCode::KeyD => 0x44,
        KeyCode::KeyE => 0x45,
        KeyCode::KeyF => 0x46,
        KeyCode::KeyG => 0x47,
        KeyCode::KeyH => 0x48,
        KeyCode::KeyI => 0x49,
        KeyCode::KeyJ => 0x4A,
        KeyCode::KeyK => 0x4B,
        KeyCode::KeyL => 0x4C,
        KeyCode::KeyM => 0x4D,
        KeyCode::KeyN => 0x4E,
        KeyCode::KeyO => 0x4F,
        KeyCode::KeyP => 0x50,
        KeyCode::KeyQ => 0x51,
        KeyCode::KeyR => 0x52,
        KeyCode::KeyS => 0x53,
        KeyCode::KeyT => 0x54,
        KeyCode::KeyU => 0x55,
        KeyCode::KeyV => 0x56,
        KeyCode::KeyW => 0x57,
        KeyCode::KeyX => 0x58,
        KeyCode::KeyY => 0x59,
        KeyCode::KeyZ => 0x5A,
        KeyCode::Numpad0 => 0x60,
        KeyCode::Numpad1 => 0x61,
        KeyCode::Numpad2 => 0x62,
        KeyCode::Numpad3 => 0x63,
        KeyCode::Numpad4 => 0x64,
        KeyCode::Numpad5 => 0x65,
        KeyCode::Numpad6 => 0x66,
        KeyCode::Numpad7 => 0x67,
        KeyCode::Numpad8 => 0x68,
        KeyCode::Numpad9 => 0x69,
        KeyCode::NumpadMultiply => 0x6A,
        KeyCode::NumpadAdd => 0x6B,
        KeyCode::NumpadSubtract => 0x6D,
        KeyCode::NumpadDecimal => 0x6E,
        KeyCode::NumpadDivide => 0x6F,
        KeyCode::F1 => 0x70,
        KeyCode::F2 => 0x71,
        KeyCode::F3 => 0x72,
        KeyCode::F4 => 0x73,
        KeyCode::F5 => 0x74,
        KeyCode::F6 => 0x75,
        KeyCode::F7 => 0x76,
        KeyCode::F8 => 0x77,
        KeyCode::F9 => 0x78,
        KeyCode::F10 => 0x79,
        KeyCode::F11 => 0x7A,
        KeyCode::F12 => 0x7B,
        KeyCode::NumLock => 0x90,
        KeyCode::ScrollLock => 0x91,
        KeyCode::Semicolon => 0xBA,
        KeyCode::Equal => 0xBB,
        KeyCode::Comma => 0xBC,
        KeyCode::Minus => 0xBD,
        KeyCode::Period => 0xBE,
        KeyCode::Slash => 0xBF,
        KeyCode::Backquote => 0xC0,
        KeyCode::BracketLeft => 0xDB,
        KeyCode::Backslash => 0xDC,
        KeyCode::BracketRight => 0xDD,
        KeyCode::Quote => 0xDE,
        _ => return None,
    })
}

/// The settings-table code for a mouse button: the primary, secondary and
/// middle buttons are 1, 2 and 4; the back and forward buttons are the
/// fourth and fifth (the mouse flag `0x8000` with 4 or 5); other buttons are
/// the sixth (with 6).
pub fn mouse_code(button: MouseButton) -> u32 {
    match button {
        MouseButton::Left => 0x01,
        MouseButton::Right => 0x02,
        MouseButton::Middle => 0x04,
        MouseButton::Back => 0x8004,
        MouseButton::Forward => 0x8005,
        MouseButton::Other(_) => 0x8006,
    }
}

#[cfg(test)]
mod rebind_tests {
    use super::*;

    #[test]
    fn the_keyboard_codes() {
        assert_eq!(virtual_key(KeyCode::KeyW), Some(0x57));
        assert_eq!(virtual_key(KeyCode::Tab), Some(0x09));
        assert_eq!(virtual_key(KeyCode::ShiftRight), Some(0x10));
        assert_eq!(virtual_key(KeyCode::ArrowLeft), Some(0x25));
        assert_eq!(virtual_key(KeyCode::F12), Some(0x7B));
        assert_eq!(virtual_key(KeyCode::F24), None);
        assert_eq!(virtual_key(KeyCode::MediaPlayPause), None);
    }

    #[test]
    fn the_mouse_codes() {
        assert_eq!(mouse_code(MouseButton::Left), 0x01);
        assert_eq!(mouse_code(MouseButton::Right), 0x02);
        assert_eq!(mouse_code(MouseButton::Middle), 0x04);
        assert_eq!(mouse_code(MouseButton::Back), 0x8004);
        assert_eq!(mouse_code(MouseButton::Forward), 0x8005);
        assert_eq!(mouse_code(MouseButton::Other(9)), 0x8006);
    }

    #[test]
    fn a_binding_shows_its_key_name() {
        let mut bindings = Bindings::default();
        assert_eq!(bindings.key_name(Action::Forward), "W");
        assert_eq!(bindings.key_name(Action::Primary), "Left mouse");
        bindings.set_key(Action::Primary, 0x8004);
        assert_eq!(bindings.key_name(Action::Primary), "Mouse 3");
        bindings.set_key(Action::Forward, 0x4000 | 0x41);
        assert_eq!(bindings.key_name(Action::Forward), "<65>");
    }
}
