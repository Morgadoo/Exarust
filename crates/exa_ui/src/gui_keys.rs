//! The window procedure's key words: the key table and the modifier mask
//! that go with every key the GUI's focused form hears.
//!
//! A key-down sends the key and the mask to the focused form's key slot
//! ([`crate::gui_core::GuiTask::key`]). The mask is the system's word the
//! GUI keeps: the first key-down of a key, with Shift, Ctrl or Alt held,
//! recomputes it as Shift 0x20, Ctrl 0x40 and Alt 0x80; a key-down with
//! none held keeps the word as it was. A repeat (the key already down)
//! sends the kept word with 1 and recomputes nothing. A key-up clears the
//! key and recomputes the word one bit lower: Shift 0x10, Ctrl 0x20, Alt
//! 0x40. A typed character above 0x1f goes to the focused form with the
//! word 2. So Shift with Tab sends 0x20, which the base form's Tab
//! ([`crate::form::form_key`], backward on 0x10) takes as a forward step;
//! only a repeat after a key-up with Shift still held carries 0x10.

/// The mask bit a repeated key-down carries.
pub const REPEAT: u32 = 1;
/// The word a typed character carries.
pub const TYPED: u32 = 2;
/// Shift, Ctrl and Alt as a key-down's mask holds them.
pub const SHIFT: u32 = 0x20;
pub const CTRL: u32 = 0x40;
pub const ALT: u32 = 0x80;

/// The virtual keys of the three modifiers.
const VK_SHIFT: usize = 0x10;
const VK_CONTROL: usize = 0x11;
const VK_MENU: usize = 0x12;

/// One key the focused form hears: the virtual key (or the typed byte) and
/// its mask word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuiKey {
    pub key: u32,
    pub flags: u32,
}

/// The key table (one byte per virtual key) and the modifier mask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMask {
    down: [bool; 256],
    pub mask: u32,
}

impl Default for KeyMask {
    fn default() -> Self {
        Self {
            down: [false; 256],
            mask: 0,
        }
    }
}

impl KeyMask {
    fn held(&self, vk: usize) -> u32 {
        u32::from(self.down[vk])
    }

    /// Whether a virtual key is down.
    pub fn is_down(&self, vk: u32) -> bool {
        self.down[(vk & 0xff) as usize]
    }

    /// A key-down of `vk`: the key the focused form hears.
    pub fn key_down(&mut self, vk: u32) -> GuiKey {
        let i = (vk & 0xff) as usize;
        if self.down[i] {
            return GuiKey {
                key: vk,
                flags: self.mask | REPEAT,
            };
        }
        self.down[i] = true;
        if self.down[VK_SHIFT] || self.down[VK_CONTROL] || self.down[VK_MENU] {
            self.mask =
                self.held(VK_SHIFT) << 5 | self.held(VK_CONTROL) << 6 | self.held(VK_MENU) << 7;
        }
        GuiKey {
            key: vk,
            flags: self.mask,
        }
    }

    /// A key-up of `vk`: the key leaves the table and the mask is
    /// recomputed one bit lower.
    pub fn key_up(&mut self, vk: u32) {
        self.down[(vk & 0xff) as usize] = false;
        self.mask = self.held(VK_SHIFT) << 4 | self.held(VK_CONTROL) << 5 | self.held(VK_MENU) << 6;
    }

    /// A typed character: only bytes above 0x1f reach the focused form.
    pub fn typed(byte: u32) -> Option<GuiKey> {
        (byte > 0x1f).then_some(GuiKey {
            key: byte,
            flags: TYPED,
        })
    }

    /// The window lost or gained the system's focus: the key table clears.
    pub fn clear(&mut self) {
        self.down = [false; 256];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_with_tab_sends_0x20_and_a_repeat_adds_1() {
        let mut keys = KeyMask::default();
        assert_eq!(keys.key_down(9).flags, 0);
        keys.key_up(9);
        assert_eq!(keys.key_down(0x10).flags, SHIFT);
        assert_eq!(keys.key_down(9).flags, SHIFT);
        assert_eq!(keys.key_down(9).flags, SHIFT | REPEAT);
        assert_eq!(keys.key_down(0x11).flags, SHIFT | CTRL);
    }

    #[test]
    fn a_key_up_recomputes_the_mask_one_bit_lower() {
        let mut keys = KeyMask::default();
        keys.key_down(0x10);
        keys.key_down(9);
        keys.key_down(0x41);
        keys.key_up(0x41);
        // Shift still held: the key-up's word has Shift as 0x10, which a
        // repeat of the held Tab carries; a fresh key-down recomputes 0x20.
        assert_eq!(keys.mask, 0x10);
        assert_eq!(keys.key_down(9).flags, 0x10 | REPEAT);
        assert_eq!(keys.key_down(0x42).flags, SHIFT);
        keys.key_up(0x10);
        keys.key_up(9);
        keys.key_up(0x42);
        assert_eq!(keys.mask, 0);
        // No modifier held: the key-down keeps the word.
        keys.mask = 0x10;
        assert_eq!(keys.key_down(0x43).flags, 0x10);
    }

    #[test]
    fn only_typed_bytes_above_0x1f_reach_the_form() {
        assert_eq!(KeyMask::typed(0x1f), None);
        assert_eq!(
            KeyMask::typed(u32::from(b'a')),
            Some(GuiKey {
                key: 0x61,
                flags: TYPED
            })
        );
    }
}
