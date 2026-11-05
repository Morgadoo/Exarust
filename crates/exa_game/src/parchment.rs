//! The map's parchment: the item card builder, asked for a card of an item
//! whose record holds the map section, opens a borderless popup showing the
//! image that section names instead (`Map_Exanima01` for the base tier's
//! "Map"). It is the card's request that opens it: a right press on the
//! map in an item panel, the inventory figure's card or a world object's
//! card, through [`crate::item_card`].
//!
//! - A parchment already open for the same item is left alone (the opener
//!   searches the open forms for the parchment's tag and that item).
//! - The image is the texture of that name; the popup's size is the
//!   texture's in sixths of the interface unit
//!   ([`exa_ui::popups::parchment_size`]).
//! - It is placed as an item card is: centred across on the pointer and
//!   kept on the screen, its bottom eight units below the pointer, never
//!   above one unit ([`exa_items::item_gui::popup_place`]).
//! - It fades in over the card's 256 ms, as the card does.
//! - A right press on it (its texture's alpha at or above 0xF0 under the
//!   pointer, [`exa_ui::popups::parchment_hits`]) fades it out, and the
//!   press goes no further. Other presses go to the base form.
//! - The draw is one white textured quad at the fade's alpha
//!   ([`exa_ui::popups::parchment_draw`]).
//!
//! Evidence: the item GUI's card builder and its parchment branch, the
//! parchment's constructor, mouse, hit and draw slots (`docs/UI.md`, item
//! GUI). The hit's sample is the texture's own pixel here; the original
//! reads the compressed block that holds it. [approximation]

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use exa_items::item_gui::popup_place;
use exa_ui::ScriptedPointer;
use exa_ui::gui_draw::{BLEND_ALPHA, GuiFrames, GuiLayer, GuiTexture, GuiTextures, LayerTexture};
use exa_ui::popups::{
    ParchmentMouse, PopupFade, parchment_draw, parchment_hits, parchment_mouse, parchment_size,
    parchment_texel,
};

/// The name the shown parchment's texture is kept under in the interface's
/// textures. One parchment is shown at a time.
pub const PARCHMENT_TEXTURE: &str = "item parchment";

/// A parchment the card builder asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenParchment {
    /// The map item's id (tier tag included).
    pub item: u32,
    /// The texture its map section names.
    pub image: String,
    /// The pointer in physical pixels.
    pub pointer: (i32, i32),
    /// The interface unit the card would have been built at.
    pub unit: i32,
    /// Asked by a world object's card, which no item panel holds.
    pub world: bool,
}

/// A shown or fading parchment.
#[derive(Debug, Clone)]
pub struct ShownParchment {
    pub item: u32,
    pub image: String,
    /// Left, top, width and height in physical pixels.
    pub rect: (i32, i32, i32, i32),
    /// The texture's own pixel size.
    pub texels: (u32, u32),
    pub unit: i32,
    pub fade: PopupFade,
}

impl ShownParchment {
    /// Whether a point hits the parchment: inside its rectangle and over a
    /// texel whose alpha is at or above 0xF0, read through the original's
    /// six texels to the unit.
    pub fn hit(&self, point: (i32, i32), texture: &GuiTexture) -> bool {
        let (x, y, w, h) = self.rect;
        if point.0 < x || point.1 < y || point.0 >= x + w || point.1 >= y + h {
            return false;
        }
        let (tx, ty) = parchment_texel((point.0 - x, point.1 - y), self.unit);
        parchment_hits(texel_alpha(texture, tx, ty))
    }
}

/// The alpha of the texel at `(x, y)` counted from the image's top, as the
/// original's sampler turns the click's row into its bottom-up storage row
/// (the height less the row). Outside the image it is transparent.
fn texel_alpha(texture: &GuiTexture, x: i32, y: i32) -> u8 {
    let (w, h) = (texture.width as i64, texture.height as i64);
    let (x, y) = (i64::from(x), i64::from(y));
    if x < 0 || y < 0 || x >= w || y >= h {
        return 0;
    }
    texture.pixels[(y * w + x) as usize][3]
}

/// The parchment and its pending request.
#[derive(Resource, Default)]
pub struct ParchmentView {
    pub shown: Option<ShownParchment>,
    /// This frame's right press was the parchment's.
    pub took_press: bool,
}

/// The pointer in physical pixels: the scripted one, else the window's.
fn pointer_at(window: &Window, scripted: Option<&ScriptedPointer>) -> Option<(i32, i32)> {
    let at = scripted
        .and_then(|s| s.0)
        .or_else(|| window.cursor_position())?
        * window.scale_factor();
    Some((at.x as i32, at.y as i32))
}

/// A right press over the parchment fades it out and is taken; any other
/// press goes on to the forms under it.
pub(crate) fn parchment_press(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scripted: Option<Res<ScriptedPointer>>,
    textures: Option<Res<GuiTextures>>,
    time: Res<Time>,
    mut view: ResMut<ParchmentView>,
) {
    view.took_press = false;
    if !buttons.just_pressed(MouseButton::Right) {
        return;
    }
    let (Ok(window), Some(textures)) = (windows.single(), textures.as_deref()) else {
        return;
    };
    let Some(point) = pointer_at(window, scripted.as_deref()) else {
        return;
    };
    let Some(texture) = textures.get(PARCHMENT_TEXTURE) else {
        return;
    };
    let now = time.elapsed().as_millis() as u32;
    let Some(shown) = view.shown.as_mut() else {
        return;
    };
    if !shown.hit(point, texture) {
        return;
    }
    if parchment_mouse(0x1002) == ParchmentMouse::Hide {
        shown.fade.fade_out(now);
        info!("parchment: {:#x} fades on a right press", shown.item);
    }
    view.took_press = true;
}

/// Opens the parchment the card builder asked for.
pub(crate) fn open_parchment(
    mut cards: ResMut<crate::item_card::CardView>,
    mut view: ResMut<ParchmentView>,
    mut textures: Option<ResMut<GuiTextures>>,
    game: Option<Res<exa_assets::Game>>,
    screens: Res<crate::gui_screens::GameScreens>,
    time: Res<Time>,
) {
    let Some(ask) = cards.parchment.take() else {
        return;
    };
    // A parchment already open for this item stays as it is.
    if view
        .shown
        .as_ref()
        .is_some_and(|shown| shown.item == ask.item && shown.fade.state != 2)
    {
        debug!("parchment: {:#x} is already open", ask.item);
        return;
    }
    let (Some(game), Some(textures)) = (game.as_deref(), textures.as_deref_mut()) else {
        return;
    };
    let Some(texture) = load_image(game, &ask.image) else {
        warn!("parchment: the map image {:?} did not load", ask.image);
        return;
    };
    let texels = (texture.width, texture.height);
    let size = parchment_size(
        texels.0.min(u32::from(u16::MAX)) as u16,
        texels.1.min(u32::from(u16::MAX)) as u16,
        ask.unit,
    );
    let at = popup_place(ask.pointer, size, screens.task.screen.0, ask.unit);
    textures
        .textures
        .insert(PARCHMENT_TEXTURE.to_string(), texture);
    let now = time.elapsed().as_millis() as u32;
    let mut fade = PopupFade::default();
    fade.show(now);
    fade.shown_at = fade.fade_at;
    info!(
        "parchment: {:#x} opens {:?} ({}x{} texels) at {at:?}, {}x{} at unit {}{}",
        ask.item,
        ask.image,
        texels.0,
        texels.1,
        size.0,
        size.1,
        ask.unit,
        if ask.world { ", a world object's" } else { "" }
    );
    view.shown = Some(ShownParchment {
        item: ask.item,
        image: ask.image,
        rect: (at.0, at.1, size.0, size.1),
        texels,
        unit: ask.unit,
        fade,
    });
}

/// The image a map names, decoded to the interface's top-down order.
fn load_image(game: &exa_assets::Game, name: &str) -> Option<GuiTexture> {
    let bytes = game.0.texture(name)?;
    let rfi = exa_formats::rfi::Rfi::parse(bytes).ok()?;
    let rgba = rfi.decode_rgba8().ok()?;
    GuiTexture::from_bottom_up(rfi.header.width, rfi.header.height, &rgba)
}

/// Draws the parchment at its fade, and drops it once its fade-out ends.
pub(crate) fn draw_parchment(
    mut view: ResMut<ParchmentView>,
    mut frames: ResMut<GuiFrames>,
    time: Res<Time>,
    menu: Option<Res<crate::menu_screen::MenuScreen>>,
) {
    let now = time.elapsed().as_millis() as u32;
    let Some(shown) = view.shown.as_mut() else {
        return;
    };
    let step = shown.fade.step(now);
    if step.close {
        info!("parchment: {:#x} closes", shown.item);
        view.shown = None;
        return;
    }
    if menu.is_some_and(|menu| menu.open) {
        return;
    }
    let alpha = (step.alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    let Some(draw) = parchment_draw(shown.rect, alpha, false, || true, false) else {
        return;
    };
    let mut batch = draw.batch;
    batch.blend = BLEND_ALPHA;
    batch.tex_col = Some(1.0);
    let mut frame = exa_ui::gui_draw::GuiFrame::new();
    frame.push(GuiLayer::new(batch, LayerTexture::Named(PARCHMENT_TEXTURE)));
    frames.push(frame);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texture() -> GuiTexture {
        // Four by two texels: the top row opaque, the bottom one clear.
        let mut pixels = vec![[0u8, 0, 0, 0xff]; 4];
        pixels.extend(vec![[0u8, 0, 0, 0]; 4]);
        GuiTexture {
            width: 4,
            height: 2,
            pixels,
        }
    }

    fn shown(unit: i32) -> ShownParchment {
        ShownParchment {
            item: 9,
            image: "Map_Exanima01".into(),
            rect: (100, 50, 4, 2),
            texels: (4, 2),
            unit,
            fade: PopupFade::default(),
        }
    }

    /// At six pixels to the unit one screen pixel is one texel: the top
    /// row hits, the clear bottom row and a point off the form do not.
    #[test]
    fn the_parchment_hits_where_its_texture_is_opaque() {
        let t = texture();
        let p = shown(6);
        assert!(p.hit((100, 50), &t));
        assert!(p.hit((103, 50), &t));
        assert!(!p.hit((101, 51), &t));
        assert!(!p.hit((99, 50), &t));
        assert!(!p.hit((104, 50), &t));
    }

    /// The map's 1024 by 512 image at unit four is 683 by 341, centred on
    /// the pointer and its bottom eight units below it, kept on screen.
    #[test]
    fn the_parchment_is_placed_as_a_card() {
        let size = parchment_size(1024, 512, 4);
        assert_eq!(size, (683, 341));
        assert_eq!(popup_place((640, 400), size, 1280, 4), (299, 91));
        assert_eq!(popup_place((10, 100), size, 1280, 4), (0, 4));
        assert_eq!(popup_place((1270, 400), size, 1280, 4), (597, 91));
    }
}
