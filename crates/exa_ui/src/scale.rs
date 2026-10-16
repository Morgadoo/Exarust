//! The interface scale: the original sizes its interface by a whole-number
//! GUI scale picked from the window's height in physical pixels, offset by
//! its GUI size setting. Rules: `docs/UI.md` (Interface scale).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// The GUI scales the original offers, by index.
const SCALES: [u32; 6] = [3, 4, 5, 6, 8, 12];
/// The window heights (physical pixels) from which the index steps up.
const HEIGHT_STEPS: [u32; 5] = [700, 870, 1051, 1390, 2080];

/// The interface scale: `size` is the original's GUI size setting, an
/// offset to the index the window's height gives (0 by default); `scale`
/// is the scale in use, kept up to date with the window.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuiScale {
    pub size: i32,
    pub scale: u32,
}

impl Default for GuiScale {
    fn default() -> Self {
        Self {
            size: 0,
            scale: gui_scale(900, 0),
        }
    }
}

/// The GUI scale for a window `height` physical pixels tall with the GUI
/// size setting `size`: 3, 4, 5, 6, 8 or 12.
pub fn gui_scale(height: u32, size: i32) -> u32 {
    let steps = HEIGHT_STEPS.iter().filter(|&&h| height >= h).count() as i32;
    SCALES[(steps + size).clamp(0, SCALES.len() as i32 - 1) as usize]
}

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<GuiScale>()
        .add_systems(PreUpdate, follow_window);
}

/// Picks the scale for the window's current height.
fn follow_window(windows: Query<&Window, With<PrimaryWindow>>, mut gui: ResMut<GuiScale>) {
    let Ok(window) = windows.single() else { return };
    let scale = gui_scale(window.physical_height(), gui.size);
    if gui.scale != scale {
        gui.scale = scale;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_height_picks_the_scale() {
        // Exarust's default 900-pixel window and the user's retail window
        // (1014 pixels tall) both give 5.
        assert_eq!(gui_scale(900, 0), 5);
        assert_eq!(gui_scale(1014, 0), 5);
        for (height, scale) in [
            (480, 3),
            (699, 3),
            (700, 4),
            (869, 4),
            (870, 5),
            (1050, 5),
            (1051, 6),
            (1440, 8),
            (2079, 8),
            (2160, 12),
        ] {
            assert_eq!(gui_scale(height, 0), scale, "{height}");
        }
    }

    #[test]
    fn the_size_setting_offsets_the_index() {
        assert_eq!(gui_scale(900, 1), 6);
        assert_eq!(gui_scale(900, -1), 4);
        assert_eq!(gui_scale(900, -9), 3);
        assert_eq!(gui_scale(900, 9), 12);
    }
}
