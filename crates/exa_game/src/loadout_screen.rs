//! The arena loadout editor as a running screen: the editor for one
//! fighter and rank, its presses (an item dragged out of the arsenal is
//! equipped, one taken off the kit is released), the close that saves the
//! kit, and the paint through the GUI draw path.
//!
//! The rules are [`exa_ui::loadout`]: the drop handler sorts a drag into its
//! container, a champion's form takes nothing, and the close writes the
//! kit in the arena and match modes. The screen holds the editor and its
//! size; the caller that opened it keeps the member and writes the kit back
//! when the close says so ([`LoadoutClose`]).
//!
//! The original's two containers and the character preview draw
//! themselves; until the control toolkit drives them the screen paints the
//! form's parts as the layout's flat fills, which is the paint's own order
//! ([`exa_ui::loadout::draw_order`]).

use bevy::prelude::*;
use exa_ui::gui_draw::GuiFrames;
use exa_ui::loadout::{
    self, DropResult, DropSource, LoadoutEditor, LoadoutInks, LoadoutLayout, LoadoutSave,
};
use exa_ui::skin::SkinPalette;

/// Open the editor for a roster member. The editor is built by the caller
/// ([`crate::loadout::editor_for`]); `wide` is the interface's wide
/// layout, which sets the form's opacity and the equipment's scale.
#[derive(Message, Debug, Clone)]
pub struct OpenLoadoutEditor {
    pub member: usize,
    pub editor: LoadoutEditor,
    pub wide: bool,
}

/// The pointer pressed an item: its id and whether the press came from the
/// arsenal (a drag onto the kit) or from the kit (a release).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadoutPress {
    pub id: u32,
    pub from_arsenal: bool,
}

/// The form's close: the caller writes the kit back for this member and
/// rank when [`LoadoutSave::saves_kit`] is set.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadoutClose {
    pub member: usize,
    pub rank: usize,
    pub save: LoadoutSave,
}

/// The form was dismissed without saving (a champion's close or a
/// creation form's).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadoutDismiss {
    pub member: usize,
}

/// The open editor.
#[derive(Debug, Clone)]
pub struct OpenLoadout {
    pub member: usize,
    pub editor: LoadoutEditor,
    pub wide: bool,
    pub width: i32,
    pub height: i32,
}

impl OpenLoadout {
    /// The form's layout for the shown width.
    pub fn layout(&self) -> LoadoutLayout {
        loadout::layout(self.editor.mode, self.width, self.wide)
    }
}

/// The open loadout form, if one is.
#[derive(Resource, Debug, Default)]
pub struct LoadoutScreen {
    pub open: Option<OpenLoadout>,
}

impl LoadoutScreen {
    /// Opens a form, replacing any other (the original finds its form
    /// already open).
    pub fn open(message: OpenLoadoutEditor) -> Self {
        Self {
            open: Some(OpenLoadout {
                member: message.member,
                editor: message.editor,
                wide: message.wide,
                width: 0,
                height: 0,
            }),
        }
    }

    /// Routes a press: an arsenal item is dropped onto the kit, an item of
    /// the kit is released. Returns what the drop did, if the form is open.
    pub fn press(&mut self, press: LoadoutPress) -> Option<DropResult> {
        let open = self.open.as_mut()?;
        if press.from_arsenal {
            let item = *open
                .editor
                .arsenal
                .iter()
                .find(|item| item.id == press.id)?;
            Some(loadout::apply_drop(
                &mut open.editor,
                &item,
                DropSource::Arsenal,
            ))
        } else {
            let item = open
                .editor
                .weapons
                .iter()
                .chain(open.editor.armour.iter())
                .find(|item| item.id == press.id)
                .copied()?;
            let source = if open.editor.weapons.iter().any(|held| held.id == item.id) {
                DropSource::Weapons
            } else {
                DropSource::Armour
            };
            // A press on the kit takes the item off it (the release).
            loadout::unequip(&mut open.editor, item.id);
            Some(DropResult {
                accepted: false,
                swapped: false,
                source,
                origin: loadout::DropOrigin::Equipment,
                off_hand: item.flags & loadout::OFFHAND_FLAG != 0,
                armed: 0,
                refresh: true,
                clear_pointer: true,
            })
        }
    }

    /// Closes the form, with what the close saves.
    pub fn close(&mut self) -> Option<LoadoutClose> {
        let open = self.open.take()?;
        Some(LoadoutClose {
            member: open.member,
            rank: open.editor.rank,
            save: loadout::save_on_close(&open.editor),
        })
    }

    /// Dismisses the form without saving.
    pub fn dismiss(&mut self) -> Option<LoadoutDismiss> {
        let open = self.open.take()?;
        Some(LoadoutDismiss {
            member: open.member,
        })
    }

    /// One frame: the backdrop, the arsenal, the equipment and the
    /// preview at the form's layout. `None` when nothing is open.
    pub fn frame(&self, palette: &SkinPalette) -> Option<exa_ui::gui_draw::GuiFrame> {
        let open = self.open.as_ref()?;
        let layout = open.layout();
        let inks = LoadoutInks::from_palette(palette);
        Some(loadout::frame(
            &loadout::draw_order(),
            &layout,
            &inks,
            open.width,
            open.height,
        ))
    }
}

/// The player asked the form to close: `dismiss` closes it without saving
/// (a champion's or creation form), otherwise the close saves.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadoutCloseRequest {
    pub dismiss: bool,
}

pub struct LoadoutScreenPlugin;

impl Plugin for LoadoutScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadoutScreen>()
            .add_message::<OpenLoadoutEditor>()
            .add_message::<LoadoutPress>()
            .add_message::<LoadoutCloseRequest>()
            .add_message::<LoadoutClose>()
            .add_message::<LoadoutDismiss>()
            .add_message::<LoadoutDropped>()
            .add_systems(
                Update,
                (open_form, press_form, close_form, paint_form).chain(),
            );
    }
}

fn open_form(mut opened: MessageReader<OpenLoadoutEditor>, mut screen: ResMut<LoadoutScreen>) {
    if screen.open.is_some() {
        return;
    }
    if let Some(message) = opened.read().next() {
        *screen = LoadoutScreen::open(message.clone());
    }
}

fn press_form(
    mut presses: MessageReader<LoadoutPress>,
    mut screen: ResMut<LoadoutScreen>,
    mut dropped: MessageWriter<LoadoutDropped>,
) {
    for press in presses.read() {
        if let Some(result) = screen.press(*press) {
            dropped.write(LoadoutDropped {
                press: *press,
                done: result,
            });
        }
    }
}

/// A press reached the kit: what the drop did.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct LoadoutDropped {
    pub press: LoadoutPress,
    pub done: DropResult,
}

fn close_form(
    mut requests: MessageReader<LoadoutCloseRequest>,
    mut screen: ResMut<LoadoutScreen>,
    mut closed: MessageWriter<LoadoutClose>,
    mut dismissed: MessageWriter<LoadoutDismiss>,
) {
    for request in requests.read() {
        if request.dismiss {
            if let Some(message) = screen.dismiss() {
                dismissed.write(message);
            }
        } else if let Some(message) = screen.close() {
            closed.write(message);
        }
    }
}

fn paint_form(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut screen: ResMut<LoadoutScreen>,
    mut frames: Option<ResMut<GuiFrames>>,
) {
    let Some(open) = screen.open.as_mut() else {
        return;
    };
    if let Ok(window) = windows.single() {
        open.width = window.physical_width() as i32;
        open.height = window.physical_height() as i32;
    }
    if open.width <= 0 || open.height <= 0 {
        return;
    }
    let palette = exa_ui::line_skin::line_palette();
    let Some(frame) = screen.frame(&palette) else {
        return;
    };
    if let Some(frames) = frames.as_mut() {
        frames.push(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_ui::loadout::{LoadoutItem, LoadoutMode};

    fn item(id: u32) -> LoadoutItem {
        LoadoutItem::weapon(id, 0x20, 2, 0)
    }

    fn editor() -> LoadoutEditor {
        LoadoutEditor {
            mode: LoadoutMode::Arena,
            rank: 1,
            limit: 10,
            max_rank: 2,
            arsenal: vec![item(1), item(2)],
            ..LoadoutEditor::default()
        }
    }

    #[test]
    fn a_press_on_the_arsenal_equips_and_a_press_on_the_kit_releases() {
        let mut screen = LoadoutScreen::open(OpenLoadoutEditor {
            member: 3,
            editor: editor(),
            wide: false,
        });
        let result = screen
            .press(LoadoutPress {
                id: 1,
                from_arsenal: true,
            })
            .unwrap();
        assert!(result.accepted);
        assert_eq!(screen.open.as_ref().unwrap().editor.weapons.len(), 1);
        assert_eq!(screen.open.as_ref().unwrap().editor.points, 2);
        // A second press takes the arsenal's other copy into the free hand;
        // the list model keeps one entry per id.
        let result = screen
            .press(LoadoutPress {
                id: 1,
                from_arsenal: true,
            })
            .unwrap();
        assert!(result.accepted);
        assert_eq!(screen.open.as_ref().unwrap().editor.weapons.len(), 1);
        assert_eq!(screen.open.as_ref().unwrap().editor.points, 2);
        let result = screen
            .press(LoadoutPress {
                id: 1,
                from_arsenal: false,
            })
            .unwrap();
        assert_eq!(result.source, DropSource::Weapons);
        assert!(screen.open.as_ref().unwrap().editor.weapons.is_empty());
        // Unknown items do nothing.
        assert!(
            screen
                .press(LoadoutPress {
                    id: 9,
                    from_arsenal: true
                })
                .is_none()
        );
    }

    #[test]
    fn the_close_saves_the_arena_kit_and_the_dismiss_does_not() {
        let mut screen = LoadoutScreen::open(OpenLoadoutEditor {
            member: 3,
            editor: editor(),
            wide: false,
        });
        let close = screen.close().unwrap();
        assert_eq!((close.member, close.rank), (3, 1));
        assert!(close.save.saves_kit && close.save.refresh_arena);
        assert!(screen.close().is_none());
        let mut screen = LoadoutScreen::open(OpenLoadoutEditor {
            member: 4,
            editor: LoadoutEditor {
                mode: LoadoutMode::Champion,
                ..editor()
            },
            wide: false,
        });
        let close = screen.close().unwrap();
        assert!(!close.save.saves_kit && close.save.stores_record);
        let mut screen = LoadoutScreen::open(OpenLoadoutEditor {
            member: 5,
            editor: editor(),
            wide: false,
        });
        screen.open.as_mut().unwrap().width = 1024;
        screen.open.as_mut().unwrap().height = 768;
        assert_eq!(screen.dismiss().unwrap().member, 5);
        assert!(screen.open.is_none());
    }

    #[test]
    fn the_frame_paints_the_forms_parts_for_the_shown_size() {
        let mut screen = LoadoutScreen::open(OpenLoadoutEditor {
            member: 0,
            editor: editor(),
            wide: true,
        });
        let palette = exa_ui::line_skin::line_palette();
        assert!(screen.frame(&palette).is_some());
        let open = screen.open.as_mut().unwrap();
        open.width = 1024;
        open.height = 768;
        let layout = open.layout();
        let frame = screen.frame(&palette).unwrap();
        assert_eq!(frame.layers.len(), 4);
        // The wide interface shrinks the arsenal to 412 (492 × 0.83739835,
        // 411.999..., to even) and the equipment to 548.
        assert_eq!(layout.arsenal[3], 412);
        assert_eq!(layout.equipment[3], 548);
        assert!((layout.opacity - 0.837_398_35).abs() < 1e-7);
    }
}
