//! The player's key bindings as the original stores them: a 23-slot table
//! of virtual-key codes the game reads the keys by, and the saved table it
//! is converted from (the Controls page's rows, in scan codes) with a
//! version word, a controller-mode flag and the count.
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

    /// The name the Controls page shows for `action`'s key. The primary
    /// and secondary buttons (1 and 2), which the original's page never
    /// names, are Exarust's "Left mouse" and "Right mouse"; the middle and
    /// extra buttons are named from their saved rows ("Mouse 3" to "Mouse
    /// 5"), and every other code by the original's rule.
    pub fn key_name(&self, action: Action) -> String {
        match self.key(action) {
            0x01 => "Left mouse".into(),
            0x02 => "Right mouse".into(),
            code @ 0x04..=0x06 => exa_formats::hw_config::key_name(row_code(code)),
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

    /// The saved payload, as the original's saver writes it: version 101,
    /// the controller flag, the count 23, then the saved table's 23 rows
    /// ([`row_code`] of each row's slot; row 21 is 0).
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(12 + COUNT * 4);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&u32::from(self.controller).to_le_bytes());
        out.extend_from_slice(&(COUNT as u32).to_le_bytes());
        for slot in ROW_SLOTS {
            let code = slot.map_or(0, |slot| row_code(self.keys[slot]));
            out.extend_from_slice(&code.to_le_bytes());
        }
        out
    }

    /// Reads a saved payload as the original's loader does. A version above
    /// 100 has the controller flag next and the count after it; any other
    /// (the older shape) has the count next and no flag (off). The count's
    /// rows overwrite the shipped saved table ([`DEFAULT_ROWS`]) from the
    /// top, row 21 is then cleared, and the table becomes the slots through
    /// the original's conversion ([`slot_code`]). Slot 0 keeps the primary
    /// mouse button. `None` when the payload ends before a word it needs.
    /// The original copies a count above 23 past its table; Exarust drops
    /// the rows past 23.
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let word = |i: usize| -> Option<u32> {
            bytes
                .get(i * 4..i * 4 + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        };
        let version = word(0)? as i32;
        let (controller, first) = if version > 100 {
            (word(1)? != 0, 3)
        } else {
            (false, 2)
        };
        let count = (word(first - 1)? as i32).max(0) as usize;
        let mut rows = DEFAULT_ROWS;
        for (i, row) in rows.iter_mut().enumerate().take(count) {
            *row = word(first + i)?;
        }
        rows[UNUSED_ROW] = 0;
        let mut keys = DEFAULT_KEYS;
        for (row, slot) in ROW_SLOTS.iter().enumerate() {
            if let Some(slot) = slot {
                keys[*slot] = slot_code(rows[row]);
            }
        }
        Some(Self { controller, keys })
    }
}

/// The saved table's rows (the Controls page's order) by the slot each
/// fills, as the original's conversion copies them. Row 21 fills none.
pub const ROW_SLOTS: [Option<usize>; COUNT] = [
    Some(1),
    Some(3),
    Some(2),
    Some(4),
    Some(6),
    Some(5),
    Some(8),
    Some(9),
    Some(10),
    Some(12),
    Some(11),
    Some(13),
    Some(16),
    Some(7),
    Some(14),
    Some(17),
    Some(18),
    Some(19),
    Some(22),
    Some(21),
    Some(20),
    None,
    Some(15),
];

/// The saved-table row no slot reads, which the loader clears.
pub const UNUSED_ROW: usize = 21;

/// The shipped saved table, in row order: keyboard scan codes (`0x100` for
/// an extended key) and, for the middle mouse button, `0x8000` with its
/// virtual-key code: W, S, A, D, left shift, X, middle mouse, V, left
/// control, tab, I, K, J, left alt, R, T, C, P, left arrow, right arrow, Q,
/// 0 and F.
pub const DEFAULT_ROWS: [u32; COUNT] = [
    0x11, 0x1F, 0x1E, 0x20, 0x2A, 0x2D, 0x8004, 0x2F, 0x1D, 0x0F, 0x17, 0x25, 0x24, 0x38, 0x13,
    0x14, 0x2E, 0x19, 0x14B, 0x14D, 0x10, 0x00, 0x21,
];

/// The scan codes of a US keyboard (set 1, without the extended prefix) and
/// the virtual-key code the system's scan-to-key mapping gives each, the
/// sided modifiers' own codes included. The keypad's keys map to the
/// navigation keys, as that mapping ignores Num Lock. A guess outside the
/// shipped keys, which the shipped tables confirm.
const SCAN_CODES: [(u32, u32); 83] = [
    (0x01, 0x1B),
    (0x02, 0x31),
    (0x03, 0x32),
    (0x04, 0x33),
    (0x05, 0x34),
    (0x06, 0x35),
    (0x07, 0x36),
    (0x08, 0x37),
    (0x09, 0x38),
    (0x0A, 0x39),
    (0x0B, 0x30),
    (0x0C, 0xBD),
    (0x0D, 0xBB),
    (0x0E, 0x08),
    (0x0F, 0x09),
    (0x10, 0x51),
    (0x11, 0x57),
    (0x12, 0x45),
    (0x13, 0x52),
    (0x14, 0x54),
    (0x15, 0x59),
    (0x16, 0x55),
    (0x17, 0x49),
    (0x18, 0x4F),
    (0x19, 0x50),
    (0x1A, 0xDB),
    (0x1B, 0xDD),
    (0x1C, 0x0D),
    (0x1D, 0xA2),
    (0x1E, 0x41),
    (0x1F, 0x53),
    (0x20, 0x44),
    (0x21, 0x46),
    (0x22, 0x47),
    (0x23, 0x48),
    (0x24, 0x4A),
    (0x25, 0x4B),
    (0x26, 0x4C),
    (0x27, 0xBA),
    (0x28, 0xDE),
    (0x29, 0xC0),
    (0x2A, 0xA0),
    (0x2B, 0xDC),
    (0x2C, 0x5A),
    (0x2D, 0x58),
    (0x2E, 0x43),
    (0x2F, 0x56),
    (0x30, 0x42),
    (0x31, 0x4E),
    (0x32, 0x4D),
    (0x33, 0xBC),
    (0x34, 0xBE),
    (0x35, 0xBF),
    (0x36, 0xA1),
    (0x37, 0x6A),
    (0x38, 0xA4),
    (0x39, 0x20),
    (0x3A, 0x14),
    (0x3B, 0x70),
    (0x3C, 0x71),
    (0x3D, 0x72),
    (0x3E, 0x73),
    (0x3F, 0x74),
    (0x40, 0x75),
    (0x41, 0x76),
    (0x42, 0x77),
    (0x43, 0x78),
    (0x44, 0x79),
    (0x45, 0x90),
    (0x46, 0x91),
    (0x47, 0x24),
    (0x48, 0x26),
    (0x49, 0x21),
    (0x4A, 0x6D),
    (0x4B, 0x25),
    (0x4C, 0x0C),
    (0x4D, 0x27),
    (0x4E, 0x6B),
    (0x4F, 0x23),
    (0x50, 0x28),
    (0x51, 0x22),
    (0x52, 0x2D),
    (0x53, 0x2E),
];

/// The slot code of a saved-table row, the original's conversion: a code
/// with either of the bits `0xC000` keeps its low 14 bits (a mouse button's
/// virtual-key code under `0x8000`); any other is its low byte's scan code
/// mapped to a virtual-key code, 0 when the mapping has none. The extended
/// bit is dropped, so a right control or alt reads as the left one.
pub fn slot_code(row: u32) -> u32 {
    if row & 0xC000 != 0 {
        return row & 0x3FFF;
    }
    let scan = row & 0xFF;
    SCAN_CODES
        .iter()
        .find(|(code, _)| *code == scan)
        .map_or(0, |(_, vk)| *vk)
}

/// The saved-table row Exarust writes for a slot code, the inverse of
/// [`slot_code`]: a mouse button (1, 2, 4, 5, 6) under `0x8000`, a code
/// that already has a `0xC000` bit as it is, either-side modifiers as their
/// left key's scan code, the navigation keys and the keypad's divide as
/// extended scan codes, the right modifiers as their own (extended for
/// control and alt), the keypad's digits and decimal as the keypad's scan
/// codes, and 0 for a code without a key.
pub fn row_code(code: u32) -> u32 {
    if code & 0xC000 != 0 {
        return code;
    }
    match code {
        0x01 | 0x02 | 0x04 | 0x05 | 0x06 => return 0x8000 | code,
        0x10 => return 0x2A,
        0x11 => return 0x1D,
        0x12 => return 0x38,
        0xA3 => return 0x11D,
        0xA5 => return 0x138,
        0x6F => return 0x135,
        0x6E => return 0x53,
        0x60..=0x69 => {
            const KEYPAD: [u32; 10] = [0x52, 0x4F, 0x50, 0x51, 0x4B, 0x4C, 0x4D, 0x47, 0x48, 0x49];
            return KEYPAD[(code - 0x60) as usize];
        }
        _ => {}
    }
    let extended = matches!(code, 0x21..=0x28 | 0x2D | 0x2E);
    SCAN_CODES
        .iter()
        .find(|(_, vk)| *vk == code)
        .map_or(0, |(scan, _)| if extended { 0x100 | scan } else { *scan })
}

/// The Bevy key for a Windows virtual-key code in the table. An
/// either-side modifier (0x10, 0x11, 0x12) names the left key ([`Bindings`]
/// reads it on both sides); the one-sided codes the saved table's
/// conversion gives (0xA0 to 0xA5) name their own side.
pub fn key_code(code: u32) -> Option<KeyCode> {
    Some(match code {
        0x10 | 0xA0 => KeyCode::ShiftLeft,
        0xA1 => KeyCode::ShiftRight,
        0x11 | 0xA2 => KeyCode::ControlLeft,
        0xA3 => KeyCode::ControlRight,
        0x12 | 0xA4 => KeyCode::AltLeft,
        0xA5 => KeyCode::AltRight,
        _ => {
            return KEYBOARD
                .iter()
                .find(|(_, vk)| *vk == code)
                .map(|(key, _)| *key);
        }
    })
}

/// The Bevy mouse button for a Windows virtual-key code in the table: the
/// primary, secondary and middle buttons (1, 2 and 4) and the two extra
/// buttons (5 and 6).
pub fn mouse_button(code: u32) -> Option<MouseButton> {
    Some(match code {
        0x01 => MouseButton::Left,
        0x02 => MouseButton::Right,
        0x04 => MouseButton::Middle,
        0x05 => MouseButton::Back,
        0x06 => MouseButton::Forward,
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

    fn payload(words: &[u32]) -> Vec<u8> {
        words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    /// The shipped tables are one table: the saved rows convert to the
    /// slots, the modifiers to their left keys' own codes.
    #[test]
    fn the_shipped_rows_convert_to_the_shipped_slots() {
        let mut words = vec![VERSION, 0, COUNT as u32];
        words.extend(DEFAULT_ROWS);
        let decoded = Bindings::decode(&payload(&words)).expect("loads");
        let mut expected = DEFAULT_KEYS;
        expected[Action::Run.slot()] = 0xA0;
        expected[Action::Alternate.slot()] = 0xA4;
        expected[Action::Interact.slot()] = 0xA2;
        assert_eq!(decoded.keys, expected);
        assert!(!decoded.controller);
        // The slots the Controls page's rows name are the rows' slots.
        for (row, binding) in exa_ui::settings::BINDINGS.iter().enumerate() {
            let saved = usize::from(binding.action) - 1;
            assert_eq!(ROW_SLOTS[saved], binding.slot, "row {row}");
        }
        // The shipped slots save as the shipped rows.
        let mut header = payload(&[VERSION, 0, COUNT as u32]);
        header.extend(payload(&DEFAULT_ROWS));
        assert_eq!(Bindings::default().encode(), header);
        assert_eq!(decoded.encode(), header);
    }

    #[test]
    fn a_saved_table_round_trips() {
        let mut bindings = Bindings {
            controller: true,
            ..default()
        };
        bindings.set_key(Action::Talk, 0x45);
        bindings.set_key(Action::Run, 0xA1);
        bindings.set_key(Action::Interact, 0xA3);
        bindings.set_key(Action::Journal, 0x8005);
        let bytes = bindings.encode();
        let decoded = Bindings::decode(&bytes).expect("loads");
        assert!(decoded.controller);
        assert_eq!(decoded.key(Action::Talk), 0x45);
        assert_eq!(decoded.key(Action::Run), 0xA1);
        // The extended bit is dropped: a right control reads as the left.
        assert_eq!(decoded.key(Action::Interact), 0xA2);
        assert_eq!(decoded.key(Action::Journal), 5);
        assert_eq!(decoded.key(Action::PowerPrevious), 0x25);
        assert_eq!(decoded.key(Action::Primary), 0x01);
        assert_eq!(Bindings::decode(&decoded.encode()), Some(decoded));
    }

    #[test]
    fn the_loader_reads_the_shape_by_the_version_and_clears_row_21() {
        // Version 100 or below: the count follows, no flag.
        let mut words = vec![100, COUNT as u32];
        words.extend(DEFAULT_ROWS);
        words[2 + UNUSED_ROW] = 0x30;
        let old = Bindings::decode(&payload(&words)).expect("the old shape loads");
        assert!(!old.controller);
        assert_eq!(old.key(Action::Forward), 0x57);
        // Row 21 is cleared after reading: it saves as 0.
        assert_eq!(&old.encode()[(3 + UNUSED_ROW) * 4..][..4], &[0; 4]);
        // Version 101 always has the flag: a 23 there is the flag, on.
        let mut words = vec![VERSION, COUNT as u32, 2, 0x12, 0x30];
        let short = Bindings::decode(&payload(&words)).expect("a short count loads");
        assert!(short.controller);
        // Two rows read; the others keep the shipped table's.
        assert_eq!(short.key(Action::Forward), 0x45);
        assert_eq!(short.key(Action::Back), 0x42);
        assert_eq!(short.key(Action::Left), 0x41);
        assert_eq!(short.key(Action::UsePower), 0x51);
        // A count past the table's 23 rows: the extra rows are dropped.
        words[2] = 40;
        words.truncate(3);
        words.extend(DEFAULT_ROWS);
        assert_eq!(
            Bindings::decode(&payload(&words)).map(|b| b.key(Action::Skills)),
            Some(0x4B)
        );
        // Any version loads; a payload that ends early does not.
        let mut words = vec![7, 1, 0x1E];
        assert_eq!(
            Bindings::decode(&payload(&words)).map(|b| b.key(Action::Forward)),
            Some(0x41)
        );
        words.truncate(2);
        assert_eq!(Bindings::decode(&payload(&words)), None);
        assert_eq!(Bindings::decode(&[]), None);
    }

    #[test]
    fn the_row_codes_invert_the_conversion() {
        for (scan, vk) in SCAN_CODES {
            assert_eq!(slot_code(row_code(vk)), vk, "{scan:#x}");
        }
        assert_eq!(row_code(0x25), 0x14B);
        assert_eq!(row_code(0x10), 0x2A);
        assert_eq!(row_code(0x04), 0x8004);
        assert_eq!(slot_code(0x8004), 0x04);
        assert_eq!(slot_code(0x4041), 0x41);
        assert_eq!(slot_code(0x00), 0);
        assert_eq!(row_code(0xFF), 0);
        // The keypad's digits save as the keypad's keys and read back as
        // the navigation keys, as the system's mapping gives them.
        assert_eq!(slot_code(row_code(0x64)), 0x25);
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

/// The keyboard keys with a virtual-key code of their own (the modifiers,
/// whose code depends on the side, are [`virtual_key`]'s and [`key_code`]'s
/// own cases).
const KEYBOARD: [(KeyCode, u32); 92] = [
    (KeyCode::Backspace, 0x08),
    (KeyCode::Tab, 0x09),
    (KeyCode::Enter, 0x0D),
    (KeyCode::CapsLock, 0x14),
    (KeyCode::Escape, 0x1B),
    (KeyCode::Space, 0x20),
    (KeyCode::PageUp, 0x21),
    (KeyCode::PageDown, 0x22),
    (KeyCode::End, 0x23),
    (KeyCode::Home, 0x24),
    (KeyCode::ArrowLeft, 0x25),
    (KeyCode::ArrowUp, 0x26),
    (KeyCode::ArrowRight, 0x27),
    (KeyCode::ArrowDown, 0x28),
    (KeyCode::Insert, 0x2D),
    (KeyCode::Delete, 0x2E),
    (KeyCode::Digit0, 0x30),
    (KeyCode::Digit1, 0x31),
    (KeyCode::Digit2, 0x32),
    (KeyCode::Digit3, 0x33),
    (KeyCode::Digit4, 0x34),
    (KeyCode::Digit5, 0x35),
    (KeyCode::Digit6, 0x36),
    (KeyCode::Digit7, 0x37),
    (KeyCode::Digit8, 0x38),
    (KeyCode::Digit9, 0x39),
    (KeyCode::KeyA, 0x41),
    (KeyCode::KeyB, 0x42),
    (KeyCode::KeyC, 0x43),
    (KeyCode::KeyD, 0x44),
    (KeyCode::KeyE, 0x45),
    (KeyCode::KeyF, 0x46),
    (KeyCode::KeyG, 0x47),
    (KeyCode::KeyH, 0x48),
    (KeyCode::KeyI, 0x49),
    (KeyCode::KeyJ, 0x4A),
    (KeyCode::KeyK, 0x4B),
    (KeyCode::KeyL, 0x4C),
    (KeyCode::KeyM, 0x4D),
    (KeyCode::KeyN, 0x4E),
    (KeyCode::KeyO, 0x4F),
    (KeyCode::KeyP, 0x50),
    (KeyCode::KeyQ, 0x51),
    (KeyCode::KeyR, 0x52),
    (KeyCode::KeyS, 0x53),
    (KeyCode::KeyT, 0x54),
    (KeyCode::KeyU, 0x55),
    (KeyCode::KeyV, 0x56),
    (KeyCode::KeyW, 0x57),
    (KeyCode::KeyX, 0x58),
    (KeyCode::KeyY, 0x59),
    (KeyCode::KeyZ, 0x5A),
    (KeyCode::Numpad0, 0x60),
    (KeyCode::Numpad1, 0x61),
    (KeyCode::Numpad2, 0x62),
    (KeyCode::Numpad3, 0x63),
    (KeyCode::Numpad4, 0x64),
    (KeyCode::Numpad5, 0x65),
    (KeyCode::Numpad6, 0x66),
    (KeyCode::Numpad7, 0x67),
    (KeyCode::Numpad8, 0x68),
    (KeyCode::Numpad9, 0x69),
    (KeyCode::NumpadMultiply, 0x6A),
    (KeyCode::NumpadAdd, 0x6B),
    (KeyCode::NumpadSubtract, 0x6D),
    (KeyCode::NumpadDecimal, 0x6E),
    (KeyCode::NumpadDivide, 0x6F),
    (KeyCode::F1, 0x70),
    (KeyCode::F2, 0x71),
    (KeyCode::F3, 0x72),
    (KeyCode::F4, 0x73),
    (KeyCode::F5, 0x74),
    (KeyCode::F6, 0x75),
    (KeyCode::F7, 0x76),
    (KeyCode::F8, 0x77),
    (KeyCode::F9, 0x78),
    (KeyCode::F10, 0x79),
    (KeyCode::F11, 0x7A),
    (KeyCode::F12, 0x7B),
    (KeyCode::NumLock, 0x90),
    (KeyCode::ScrollLock, 0x91),
    (KeyCode::Semicolon, 0xBA),
    (KeyCode::Equal, 0xBB),
    (KeyCode::Comma, 0xBC),
    (KeyCode::Minus, 0xBD),
    (KeyCode::Period, 0xBE),
    (KeyCode::Slash, 0xBF),
    (KeyCode::Backquote, 0xC0),
    (KeyCode::BracketLeft, 0xDB),
    (KeyCode::Backslash, 0xDC),
    (KeyCode::BracketRight, 0xDD),
    (KeyCode::Quote, 0xDE),
];

/// The Windows virtual-key code for a Bevy key, for the rebinding screen:
/// shift, control and alt take their either-side codes (0x10, 0x11, 0x12).
/// Keys without a code here are ignored: `None`.
pub fn virtual_key(code: KeyCode) -> Option<u32> {
    Some(match code {
        KeyCode::ShiftLeft | KeyCode::ShiftRight => 0x10,
        KeyCode::ControlLeft | KeyCode::ControlRight => 0x11,
        KeyCode::AltLeft | KeyCode::AltRight => 0x12,
        _ => {
            return KEYBOARD
                .iter()
                .find(|(key, _)| *key == code)
                .map(|(_, vk)| *vk);
        }
    })
}

/// The slot code the Controls page stores for a mouse button: its Windows
/// virtual-key code. The original's window procedure gives the middle
/// button bit 4 and the two extra buttons bits 8 and 0x10, and its
/// rebinding maps those to the saved rows `0x8004`, `0x8005` and `0x8006`
/// (the mouse flag over the button's virtual-key code, named "Mouse 3",
/// "Mouse 4" and "Mouse 5"), which [`slot_code`] reads back as 4, 5 and 6.
/// The primary and secondary buttons keep the shipped table's codes 1 and 2
/// (the original's rebinding ignores them). Other buttons have no code.
pub fn mouse_code(button: MouseButton) -> Option<u32> {
    Some(match button {
        MouseButton::Left => 0x01,
        MouseButton::Right => 0x02,
        MouseButton::Middle => 0x04,
        MouseButton::Back => 0x05,
        MouseButton::Forward => 0x06,
        MouseButton::Other(_) => return None,
    })
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
        assert_eq!(mouse_code(MouseButton::Left), Some(0x01));
        assert_eq!(mouse_code(MouseButton::Right), Some(0x02));
        assert_eq!(mouse_code(MouseButton::Middle), Some(0x04));
        assert_eq!(mouse_code(MouseButton::Back), Some(0x05));
        assert_eq!(mouse_code(MouseButton::Forward), Some(0x06));
        assert_eq!(mouse_code(MouseButton::Other(9)), None);
        // Saved as the original's rebinding stores them.
        assert_eq!(row_code(0x04), 0x8004);
        assert_eq!(row_code(0x05), 0x8005);
        assert_eq!(row_code(0x06), 0x8006);
    }

    #[test]
    fn a_captured_mouse_code_comes_back_to_its_button() {
        for button in [
            MouseButton::Left,
            MouseButton::Right,
            MouseButton::Middle,
            MouseButton::Back,
            MouseButton::Forward,
        ] {
            let code = mouse_code(button).unwrap();
            assert_eq!(mouse_button(code), Some(button));
            assert_eq!(mouse_button(slot_code(row_code(code))), Some(button));
        }
        assert_eq!(mouse_button(0x07), None);
    }

    #[test]
    fn the_middle_button_is_mouse_3() {
        let mut bindings = Bindings::default();
        bindings.set_key(Action::Orbit, mouse_code(MouseButton::Middle).unwrap());
        assert_eq!(bindings.key_name(Action::Orbit), "Mouse 3");
        bindings.set_key(Action::Orbit, mouse_code(MouseButton::Back).unwrap());
        assert_eq!(bindings.key_name(Action::Orbit), "Mouse 4");
        bindings.set_key(Action::Orbit, mouse_code(MouseButton::Forward).unwrap());
        assert_eq!(bindings.key_name(Action::Orbit), "Mouse 5");
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
