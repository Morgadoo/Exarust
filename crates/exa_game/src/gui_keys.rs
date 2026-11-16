//! The keys of one frame as the GUI's focused form hears them: each
//! key-down with the window procedure's mask word, then the character it
//! typed with the word 2 ([`exa_ui::gui_keys`]). The window managers read
//! the list ([`GuiKeyFrame`]) and hand each key to their focused form's
//! key slot ([`exa_ui::gui_core::GuiTask::key`]): the game screens
//! ([`crate::gui_screens::screens_keys`]), the manual and the hub's forms.
//!
//! A key the focused window does not take climbs its parents to the task.
//! In play the player's hotkeys are the game view's key handler, which a
//! key reaches only while the view has the focus (no game screen under
//! the pointer); [`crate::hotkeys`] reads
//! [`crate::gui_screens::GameScreens::keys_to_window`] for that.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use exa_ui::gui_keys::{GuiKey, KeyMask};

/// This frame's keys for the GUI, in the order the window procedure sends
/// them, and the key table and mask they were read with.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct GuiKeyFrame {
    pub keys: Vec<GuiKey>,
    pub mask: KeyMask,
}

impl GuiKeyFrame {
    /// The key-downs (not the typed characters) of this frame.
    pub fn key_downs(&self) -> impl Iterator<Item = GuiKey> + '_ {
        self.keys
            .iter()
            .copied()
            .filter(|k| k.flags & exa_ui::gui_keys::TYPED == 0)
    }
}

/// Reads the frame's keyboard messages into [`GuiKeyFrame`]: a key-down
/// through the key table and mask, then its character (not with Ctrl,
/// through the ANSI code page) as a typed byte; a key-up through the
/// table. Scripted key-downs (`--press`) come without a message and are
/// read from the button state.
pub fn read_gui_keys(
    mut events: MessageReader<KeyboardInput>,
    buttons: Option<Res<ButtonInput<KeyCode>>>,
    mut frame: ResMut<GuiKeyFrame>,
) {
    let frame = &mut *frame;
    frame.keys.clear();
    let mut seen: Vec<(KeyCode, bool)> = Vec::new();
    for event in events.read() {
        let pressed = event.state.is_pressed();
        seen.push((event.key_code, pressed));
        let Some(vk) = crate::bindings::virtual_key(event.key_code) else {
            continue;
        };
        if !pressed {
            frame.mask.key_up(vk);
            continue;
        }
        frame.keys.push(frame.mask.key_down(vk));
        if frame.mask.is_down(0x11) {
            continue;
        }
        let text = match &event.logical_key {
            Key::Character(text) => text.to_string(),
            Key::Space => " ".to_string(),
            _ => continue,
        };
        for c in text.chars() {
            if let Some(typed) =
                crate::os_clipboard::char_ansi(c).and_then(|b| KeyMask::typed(u32::from(b)))
            {
                frame.keys.push(typed);
            }
        }
    }
    let Some(buttons) = buttons else {
        return;
    };
    for &code in buttons.get_just_pressed() {
        if seen.contains(&(code, true)) {
            continue;
        }
        if let Some(vk) = crate::bindings::virtual_key(code) {
            frame.keys.push(frame.mask.key_down(vk));
        }
    }
    for &code in buttons.get_just_released() {
        if seen.contains(&(code, false)) {
            continue;
        }
        if let Some(vk) = crate::bindings::virtual_key(code) {
            frame.mask.key_up(vk);
        }
    }
}

/// Installs [`GuiKeyFrame`] and its reader, after Bevy's input.
pub struct GuiKeysPlugin;

impl Plugin for GuiKeysPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GuiKeyFrame>()
            .add_message::<KeyboardInput>()
            .add_systems(PreUpdate, read_gui_keys.after(bevy::input::InputSystems));
    }
}

/// Adds [`GuiKeysPlugin`] once, for every plugin whose window manager
/// reads the keys.
pub fn ensure(app: &mut App) {
    if !app.is_plugin_added::<GuiKeysPlugin>() {
        app.add_plugins(GuiKeysPlugin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::ButtonState;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(GuiKeysPlugin);
        app.init_resource::<ButtonInput<KeyCode>>();
        app
    }

    fn send(app: &mut App, code: KeyCode, key: Key, pressed: bool) {
        app.world_mut().write_message(KeyboardInput {
            key_code: code,
            logical_key: key,
            state: if pressed {
                ButtonState::Pressed
            } else {
                ButtonState::Released
            },
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    }

    #[test]
    fn the_frames_keys_carry_the_window_procedures_mask_and_typed_bytes() {
        let mut app = app();
        send(&mut app, KeyCode::ShiftLeft, Key::Shift, true);
        send(&mut app, KeyCode::Tab, Key::Tab, true);
        send(&mut app, KeyCode::KeyA, Key::Character("A".into()), true);
        app.update();
        let keys = app.world().resource::<GuiKeyFrame>().keys.clone();
        assert_eq!(
            keys,
            vec![
                GuiKey {
                    key: 0x10,
                    flags: 0x20
                },
                GuiKey {
                    key: 9,
                    flags: 0x20
                },
                GuiKey {
                    key: 0x41,
                    flags: 0x20
                },
                GuiKey {
                    key: u32::from(b'A'),
                    flags: 2
                },
            ]
        );
        // A key-up lowers the mask; the held Tab's repeat then carries 0x10.
        send(&mut app, KeyCode::KeyA, Key::Character("a".into()), false);
        send(&mut app, KeyCode::Tab, Key::Tab, true);
        app.update();
        let keys = app.world().resource::<GuiKeyFrame>().keys.clone();
        assert_eq!(
            keys,
            vec![GuiKey {
                key: 9,
                flags: 0x11
            }]
        );
    }
}
