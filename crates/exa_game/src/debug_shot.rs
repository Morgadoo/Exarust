//! Where the debug screenshots come from: the primary window, or, in an
//! offscreen run (a grid quadrant), the image the window's cameras draw
//! into. The offscreen run has no window of its own, so a window shot fails
//! there ("Unknown window for screenshot"). Every debug screenshot path
//! (`--screenshot`, `--searcher-shots`) takes its shot through
//! [`window_shot`]. A debug aid, not the original's.

use bevy::prelude::*;
use bevy::render::view::screenshot::Screenshot;

/// The image the window's cameras draw into in an offscreen run; set by the
/// binary's offscreen plugin.
#[derive(Resource, Debug, Clone)]
pub struct OffscreenImage(pub Handle<Image>);

/// A screenshot of what the window shows: the offscreen image when there is
/// one, else the primary window.
pub fn window_shot(offscreen: Option<&OffscreenImage>) -> Screenshot {
    match offscreen {
        Some(image) => Screenshot::image(image.0.clone()),
        None => Screenshot::primary_window(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::camera::RenderTarget;

    /// An offscreen run shoots its image, a windowed one the window.
    #[test]
    fn the_shot_follows_the_offscreen_image() {
        let image = Handle::<Image>::default();
        let shot = window_shot(Some(&OffscreenImage(image.clone())));
        assert!(matches!(shot.0, RenderTarget::Image(ref i) if i.handle == image));
        assert!(matches!(
            window_shot(None).0,
            RenderTarget::Window(bevy::window::WindowRef::Primary)
        ));
    }
}
