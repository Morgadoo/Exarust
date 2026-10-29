//! The window's own pointer: the original's twelve-slot cursor table, the
//! show and hide rule that picks a slot, and the menu cursor
//! (`menucursor.rfi`) the game builds at start-up and installs in the
//! default and text slots. Wherever the system pointer shows (menus, the
//! character view, panels), it is the menu cursor at the original's size
//! and hotspot. The in-game world cursor is drawn by [`crate::cursor`].
//! Rules: `docs/UI.md` (System pointer).

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::{
    CursorIcon, CustomCursor, CustomCursorImage, Monitor, PrimaryMonitor, PrimaryWindow,
    SystemCursorIcon,
};
use exa_assets::{Game, RgbaImage};

use crate::game_gui::{menu_cursor_hotspot, menu_cursor_size};

/// Slots in the cursor table.
pub const SLOTS: usize = 12;

/// The slot the window shows by default.
pub const DEFAULT_SLOT: usize = 0;

/// The text slot.
pub const TEXT_SLOT: usize = 8;

/// The slots the menu cursor replaces: the default and the text pointer.
pub const MENU_CURSOR_SLOTS: [usize; 2] = [DEFAULT_SLOT, TEXT_SLOT];

/// The menu cursor's picture, without its extension.
pub const MENU_CURSOR_STEM: &str = "menucursor";

/// The sharpen the menu cursor gets after its resample: amount and radius.
pub const MENU_CURSOR_SHARPEN: (f32, f32) = (0.01, 1.0);

/// The original's cursor size setting (`CursorSize`, 1 in the shipped
/// settings); Exarust does not read it yet, as the world cursor does not.
pub const CURSOR_SIZE_SETTING: f32 = 1.0;

/// What one slot of the table shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotImage {
    /// One of the platform's own pointers.
    System(SystemCursorIcon),
    /// The menu cursor built from `menucursor.rfi`.
    Menu,
    /// No image: the pointer disappears while this slot is shown.
    Empty,
}

impl SlotImage {
    /// The slots as the original loads them when its executable carries no
    /// cursor resources of its own (the shipped one carries none): arrow,
    /// up-down, left-right, the two diagonals, hand, wait, four-way move,
    /// text beam, cross, an empty "add" slot (only the resource set fills
    /// it) and the not-allowed sign.
    pub fn system_slots() -> [SlotImage; SLOTS] {
        use SystemCursorIcon as S;
        [
            SlotImage::System(S::Default),
            SlotImage::System(S::NsResize),
            SlotImage::System(S::EwResize),
            SlotImage::System(S::NwseResize),
            SlotImage::System(S::NeswResize),
            SlotImage::System(S::Pointer),
            SlotImage::System(S::Wait),
            SlotImage::System(S::Move),
            SlotImage::System(S::Text),
            SlotImage::System(S::Crosshair),
            SlotImage::Empty,
            SlotImage::System(S::NotAllowed),
        ]
    }
}

/// What a change of the shown slot does to the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerChange {
    /// The same slot was asked for again: nothing happens.
    Unchanged,
    /// A negative slot hides the pointer.
    Hide,
    /// A slot is shown; `reveal` when the pointer was hidden before.
    Show { image: SlotImage, reveal: bool },
}

/// The cursor table and the slot in force.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct CursorTable {
    slots: [SlotImage; SLOTS],
    /// The slot shown, negative while the pointer is hidden. The original
    /// starts on slot 0.
    shown: i32,
}

impl Default for CursorTable {
    fn default() -> Self {
        Self {
            slots: SlotImage::system_slots(),
            shown: DEFAULT_SLOT as i32,
        }
    }
}

impl CursorTable {
    /// Replaces a slot's image, as the game unit does for the menu cursor.
    /// The original stores any index unchecked; a slot past the table is
    /// ignored here.
    pub fn install(&mut self, slot: usize, image: SlotImage) {
        if let Some(entry) = self.slots.get_mut(slot) {
            *entry = image;
        }
    }

    /// A slot's image; past the table there is none.
    pub fn slot(&self, slot: usize) -> SlotImage {
        self.slots.get(slot).copied().unwrap_or(SlotImage::Empty)
    }

    /// The slot in force (negative while hidden).
    pub fn shown(&self) -> i32 {
        self.shown
    }

    /// Shows a slot, as the original's pointer setter does: asking for the
    /// slot already in force does nothing; a negative slot hides the
    /// pointer (the display count is driven below zero); any other slot is
    /// shown, revealing the pointer first when it was hidden.
    pub fn show(&mut self, slot: i32) -> PointerChange {
        if slot == self.shown {
            return PointerChange::Unchanged;
        }
        let was_hidden = self.shown < 0;
        self.shown = slot;
        if slot < 0 {
            return PointerChange::Hide;
        }
        PointerChange::Show {
            image: self.slot(slot as usize),
            reveal: was_hidden,
        }
    }

    /// The image the window sets when the pointer enters its client area:
    /// the slot in force, or `None` while hidden (the original then leaves
    /// the pointer alone).
    pub fn client_image(&self) -> Option<SlotImage> {
        (self.shown >= 0).then(|| self.slot(self.shown as usize))
    }
}

/// The menu cursor at its built size: top-down RGBA8 rows and the hotspot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuCursorImage {
    pub size: u32,
    pub hotspot: (u16, u16),
    pub pixels: Vec<u8>,
}

/// Builds the menu cursor as the original's game unit does: the picture
/// resampled to a square of [`menu_cursor_size`] for the screen's height
/// and the size setting, sharpened by [`MENU_CURSOR_SHARPEN`], with the
/// hotspot of [`menu_cursor_hotspot`]. `image` holds its rows bottom-up, as
/// the engine stores pictures; the result is top-down for the window.
/// `None` when the size comes out below one pixel or the picture is short.
pub fn build_menu_cursor(
    image: &RgbaImage,
    format_word: u32,
    image_flags: u32,
    screen_height: i32,
    size_setting: f32,
) -> Option<MenuCursorImage> {
    let size = menu_cursor_size(screen_height, size_setting);
    if size < 1 {
        return None;
    }
    let side = size as usize;
    let resampled = exa_render::mipmap::resample_rgba8_to_size(
        &image.pixels,
        image.width as usize,
        image.height as usize,
        side,
        side,
        format_word,
        image_flags,
    );
    let (amount, radius) = MENU_CURSOR_SHARPEN;
    let sharpened = exa_render::mipmap::sharpen_rgba8(
        &resampled,
        side,
        side,
        amount,
        radius,
        format_word,
        image_flags,
    );
    if sharpened.len() != side * side * 4 {
        return None;
    }
    let mut pixels = Vec::with_capacity(sharpened.len());
    for row in sharpened.chunks_exact(side * 4).rev() {
        pixels.extend_from_slice(row);
    }
    let (x, y) = menu_cursor_hotspot(size);
    let inside = |v: i32| v.clamp(0, size - 1) as u16;
    Some(MenuCursorImage {
        size: size as u32,
        hotspot: (inside(x), inside(y)),
        pixels,
    })
}

/// The built menu cursor and the screen height it was built for.
#[derive(Resource, Default)]
struct MenuCursor {
    built_for: Option<i32>,
    image: Option<(Handle<Image>, (u16, u16))>,
}

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<CursorTable>()
        .init_resource::<MenuCursor>()
        .add_systems(
            Update,
            (build_menu_cursor_for_screen, apply_pointer).chain(),
        );
}

/// The screen height the original sizes the menu cursor by: the primary
/// display's, else the first display's (Wayland names no primary).
fn screen_height(
    primary: &Query<&Monitor, With<PrimaryMonitor>>,
    monitors: &Query<&Monitor>,
) -> Option<i32> {
    primary
        .iter()
        .next()
        .or_else(|| monitors.iter().next())
        .map(|m| m.physical_height as i32)
}

/// Builds the menu cursor once the display is known, and again when the
/// screen height changes, and installs it in the default and text slots.
fn build_menu_cursor_for_screen(
    game: Option<Res<Game>>,
    primary: Query<&Monitor, With<PrimaryMonitor>>,
    monitors: Query<&Monitor>,
    mut menu: ResMut<MenuCursor>,
    mut table: ResMut<CursorTable>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(game) = game else { return };
    let Some(height) = screen_height(&primary, &monitors) else {
        return;
    };
    if menu.built_for == Some(height) {
        return;
    }
    menu.built_for = Some(height);
    let built =
        game.load_rgba_with_format(MENU_CURSOR_STEM)
            .ok()
            .and_then(|(image, format, flags)| {
                build_menu_cursor(&image, format, flags, height, CURSOR_SIZE_SETTING)
            });
    let Some(built) = built else {
        warn!("{MENU_CURSOR_STEM} missing or empty: the system pointer stays the platform's");
        return;
    };
    info!(
        "menu cursor: {} px, hotspot {:?}, for a {height} px tall screen",
        built.size, built.hotspot
    );
    let image = Image::new(
        Extent3d {
            width: built.size,
            height: built.size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        built.pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD,
    );
    menu.image = Some((images.add(image), built.hotspot));
    for slot in MENU_CURSOR_SLOTS {
        table.install(slot, SlotImage::Menu);
    }
}

/// Sets the window's pointer image from the slot in force, as the
/// original's window does when the pointer enters its client area. A slot
/// with no image (or the menu cursor before it is built) leaves the
/// platform's arrow.
fn apply_pointer(
    mut commands: Commands,
    table: Res<CursorTable>,
    menu: Res<MenuCursor>,
    windows: Query<(Entity, Option<&CursorIcon>), With<PrimaryWindow>>,
) {
    let Ok((window, current)) = windows.single() else {
        return;
    };
    let Some(image) = table.client_image() else {
        return;
    };
    let icon = match image {
        SlotImage::System(icon) => CursorIcon::System(icon),
        SlotImage::Menu => match &menu.image {
            Some((handle, hotspot)) => CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
                handle: handle.clone(),
                hotspot: *hotspot,
                ..default()
            })),
            None => CursorIcon::System(SystemCursorIcon::Default),
        },
        // Guess: winit has no empty pointer image; the slot falls back to
        // the platform's arrow (the shipped game never shows this slot).
        SlotImage::Empty => CursorIcon::System(SystemCursorIcon::Default),
    };
    if current != Some(&icon) {
        commands.entity(window).insert(icon);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_holds_the_platform_pointers_in_the_original_order() {
        let table = CursorTable::default();
        assert_eq!(table.shown(), 0);
        assert_eq!(table.slot(0), SlotImage::System(SystemCursorIcon::Default));
        assert_eq!(table.slot(1), SlotImage::System(SystemCursorIcon::NsResize));
        assert_eq!(table.slot(5), SlotImage::System(SystemCursorIcon::Pointer));
        assert_eq!(table.slot(7), SlotImage::System(SystemCursorIcon::Move));
        assert_eq!(table.slot(8), SlotImage::System(SystemCursorIcon::Text));
        assert_eq!(
            table.slot(9),
            SlotImage::System(SystemCursorIcon::Crosshair)
        );
        assert_eq!(table.slot(10), SlotImage::Empty);
        assert_eq!(
            table.slot(11),
            SlotImage::System(SystemCursorIcon::NotAllowed)
        );
        assert_eq!(table.slot(12), SlotImage::Empty);
    }

    #[test]
    fn showing_a_slot_hides_and_reveals_the_pointer() {
        let mut table = CursorTable::default();
        assert_eq!(table.show(0), PointerChange::Unchanged);
        assert_eq!(table.show(-1), PointerChange::Hide);
        assert_eq!(table.client_image(), None);
        assert_eq!(table.show(-1), PointerChange::Unchanged);
        assert_eq!(
            table.show(8),
            PointerChange::Show {
                image: SlotImage::System(SystemCursorIcon::Text),
                reveal: true
            }
        );
        assert_eq!(
            table.show(2),
            PointerChange::Show {
                image: SlotImage::System(SystemCursorIcon::EwResize),
                reveal: false
            }
        );
        // A hidden pointer stays hidden through a second negative slot.
        assert_eq!(table.show(-5), PointerChange::Hide);
        assert_eq!(table.shown(), -5);
    }

    #[test]
    fn the_menu_cursor_takes_the_default_and_text_slots() {
        let mut table = CursorTable::default();
        for slot in MENU_CURSOR_SLOTS {
            table.install(slot, SlotImage::Menu);
        }
        table.install(40, SlotImage::Menu);
        assert_eq!(table.client_image(), Some(SlotImage::Menu));
        assert_eq!(table.slot(8), SlotImage::Menu);
        assert_eq!(
            table.slot(9),
            SlotImage::System(SystemCursorIcon::Crosshair)
        );
    }

    /// A 2 × 2 picture, rows bottom-up, built for a 1080-pixel screen: a
    /// 32-pixel square (a magnified flat picture stays flat) with the
    /// hotspot at (6, 3); the top row of the result is the picture's top.
    #[test]
    fn the_menu_cursor_is_built_at_the_screen_size() {
        let image = RgbaImage {
            width: 2,
            height: 2,
            pixels: vec![200; 16],
        };
        let rgba8 = exa_formats::pixel::id::RGBA8;
        let built = build_menu_cursor(&image, rgba8, 0, 1080, 1.0).unwrap();
        assert_eq!(built.size, 32);
        assert_eq!(built.hotspot, (6, 3));
        assert_eq!(built.pixels.len(), 32 * 32 * 4);
        assert!(built.pixels.iter().all(|&b| b == 200));
        let big = build_menu_cursor(&image, rgba8, 0, 2160, 4.0).unwrap();
        assert_eq!((big.size, big.hotspot), (128, (23, 10)));
        assert!(build_menu_cursor(&image, rgba8, 0, 10, 1.0).is_none());
        let short = RgbaImage {
            width: 2,
            height: 2,
            pixels: vec![200; 8],
        };
        assert!(build_menu_cursor(&short, rgba8, 0, 1080, 1.0).is_none());
    }

    /// Rows come out top-down: a picture whose bottom row is black and top
    /// row white gives a white first row.
    #[test]
    fn the_menu_cursor_rows_are_flipped_for_the_window() {
        let mut pixels = vec![0u8; 8];
        pixels.extend([255u8; 8]);
        let image = RgbaImage {
            width: 2,
            height: 2,
            pixels,
        };
        // A 32-pixel target is a magnification, filtered in linear space
        // for alpha: the first row is the picture's top.
        let built = build_menu_cursor(&image, exa_formats::pixel::id::RGBA8, 0, 1080, 1.0).unwrap();
        let first_alpha = built.pixels[3];
        let last_alpha = built.pixels[built.pixels.len() - 1];
        assert!(first_alpha > last_alpha, "{first_alpha} {last_alpha}");
    }

    /// The installed `menucursor.rfi` builds a 32-pixel cursor for a
    /// 1080-pixel screen with visible pixels near its hotspot's corner
    /// (skipped without `EXANIMA_DIR`).
    #[test]
    fn the_installed_menu_cursor() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let rfi = exa_formats::rfi::Rfi::parse(game.texture(MENU_CURSOR_STEM).unwrap()).unwrap();
        let image = RgbaImage {
            width: rfi.header.width,
            height: rfi.header.height,
            pixels: rfi.decode_rgba8().unwrap(),
        };
        let built = build_menu_cursor(
            &image,
            rfi.header.format_word,
            rfi.header.flags,
            1080,
            CURSOR_SIZE_SETTING,
        )
        .unwrap();
        assert_eq!((built.size, built.hotspot), (32, (6, 3)));
        assert!(built.pixels.chunks(4).any(|p| p[3] > 128));
    }
}
