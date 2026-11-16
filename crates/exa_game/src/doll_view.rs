//! The inventory's character view: the dressed figure of the character the
//! window is bound to (the player's own, or another character's, see
//! [`GameScreens::inventory_body`]) drawn by a 3D view of its own inside the
//! window's doll panel, the encumbrance dots beside the Encumbrance label,
//! and the drop that dresses that character from a dragged item. Below,
//! "the player" is that character.
//!
//! The figure is a copy of the player's body (its model, physique, skin,
//! hair and outfit) standing at its own place far from the world, on a render layer no other camera takes. An off-screen
//! camera at the character view's pose (the figure point 100 cm up, the eye
//! 410 cm away towards (−200, 60, 360), a 30° field of view) draws it into
//! an image the size of the view, which a UI node shows under the GUI
//! canvas; the inventory's painter clears the canvas over the view so the
//! figure shows through, and the camera clears to the colour the canvas
//! had there. The view's one light shines from (0.5, 0.7, 0.5) in the
//! camera's space, as the view's task writes the scene's first light each
//! frame. The figure turns toward the pointer ([`crate::char_view`]). The
//! camera, its image and the figure exist only while the inventory is open.
//!
//! The rating-dots record (class 0xac01) shows ten times the player's
//! encumbrance, drawn with the dot sheet's whole and half dots in the
//! label group's seventh ink, the dots centred on the record's height.
//!
//! The figure's hands are the view's hand scene, built from the player's
//! record as the original's scene job builds it
//! ([`exa_items::doll::scene_plan`]): each hand whose item differs from the
//! one its model shows gets that item's model when it has one, attached as
//! a weapon (the first or second held slot's grip), a shield (the left
//! hand, turned -5° about z) or a light (the torch's grip for that hand);
//! the job's steps run in the original's order
//! ([`exa_items::doll::AttachPlan::rebuild_steps`]): the changed hands'
//! old models detach, the new ones attach, the grip is recomputed (the
//! figure's held moveset follows its hands) and the stance restarts with
//! blend 20. A hand emptied without a new item releases its old model
//! ([`exa_items::doll::release_model`]), its take-out being the drag off the
//! figure's. So a shield, a second weapon and the torch show on the figure
//! as the main-hand weapon does, and follow the record while the view is
//! open. Exarust's carried torch is outside the record: the figure holds it
//! in the hand the player holds it in when the record leaves that hand
//! empty, else in the other hand when that one is empty [Exarust's].
//!
//! A drop on the view runs the character view's drop: apparel is worn
//! through the equip conflict pass (the pieces it displaces go back to the
//! pack, the braies are dropped), a weapon is taken into the hands by the
//! take-in plan and a shield into the off hand.
//!
//! Evidence: the inventory's constructor, bind and encumbrance callback,
//! the character view's constructor, background, task and drop, the item
//! GUI's control table and rating-dots draw (`docs/UI.md`, Character view).
//! Guesses: the field of view is taken as vertical; the background (the
//! view's own still when the inventory sets its flag) is taken as the
//! inventory's own client showing through. Not drawn: a lit item's fire on
//! the figure (the torch's model shows unlit; its lit mark is logged).

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::image::CompressedImageFormatSupport;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::window::PrimaryWindow;
use exa_assets::live_objects::{Changes, LiveObjects};
use exa_assets::model::{self, ModelLoader};
use exa_assets::{Game, ModelCache, PackKind};
use exa_character::held::Hands;
use exa_character::{Garment, Outfit, Wielded};
use exa_formats::characters::{ID_MASK, ItemRef};
use exa_formats::objdb::Item;
use exa_items::catalogue::ItemCatalogue;
use exa_items::doll::{
    ApplyStep, AttachKind, IntoFacts, STANCE_BLEND, TakeOut, hand_node, record_into, release_model,
    scene_plan, take_into_hands,
};
use exa_items::possession::{self, HELD, Hand, HoldStep, Possessions};
use exa_ui::character_view::{
    DropFacts, FIELD_OF_VIEW, FIGURE_POINT, FigureDrop, default_camera_position, doll_facing,
    figure_drop,
};
use exa_ui::gui_core::GuiRect;
use exa_ui::gui_draw::{GuiLayer, GuiTexture, LayerTexture, icon_batch};
use exa_ui::icon_atlas::{IconAtlas, IconImage};

use crate::body_inventory::{self, PanelKind};
use crate::gui_screens::{self, GameScreens};
use crate::item_container::Cell;
use crate::item_panel_draw::PanelIcons;
use crate::panel_live::Panel;

/// The render layer of the figure, its camera and its light: far above the
/// layers the lights' shadow links hand out (one a light from 3), so no
/// scene camera and no other off-screen scene sees it. Exarust's choice.
pub const DOLL_LAYER: usize = 4096;
/// The view's render layers. Bevy's `const` constructor only takes the
/// layers of its inline block (under 64), so the set is built at run time.
pub fn doll_layers() -> RenderLayers {
    RenderLayers::from_layers(&[DOLL_LAYER])
}
/// Where the figure stands, metres: far below any level, so nothing of the
/// world is near it.
pub const DOLL_ORIGIN: Vec3 = Vec3::new(0.0, -1000.0, 0.0);
/// The view renders at this many pixels a view pixel on each axis, and the
/// UI node scales it down (the view's supersampling, Exarust's measure).
/// The creator's view keeps it; the inventory's takes [`doll_scale`].
pub const DOLL_SCALE: u32 = 2;

/// The inventory view's pixels per view pixel on each axis, from the
/// supersampling level its constructor asks for
/// ([`exa_ui::character_view::supersample_level`]) at the settings' value:
/// Exarust's super-sampling switch stands for the original's setting 4
/// (4x, level 4) when on and 0 (level 1) when off. A level's sub-frames
/// are drawn as a square grid of samples (Exarust's 2 x 2 for 4x), so the
/// scale is the level's whole square root: 2 on, 1 off.
pub fn doll_scale(super_sampling: bool) -> u32 {
    let level = exa_ui::character_view::supersample_level(if super_sampling { 4 } else { 0 });
    (level.max(1) as u32).isqrt()
}
/// The UI node's depth: under the GUI canvas (200), so the forms, the
/// dragged icon and the cards draw over the figure.
pub const DOLL_Z: i32 = 199;
/// The view light's illuminance: a light of colour 0.75, as the item icon
/// scene's sun (`crate::item_icon_render::ICON_SUN`). [guess at the view
/// scene's light level 1 over its 0.75 environment colour]
pub const DOLL_SUN: f32 = 120.0;
/// The drag destination number of the inventory's character view: the
/// pack's linked panel, so a drag from the pack onto the figure leaves the
/// pack without losing its bulk. Distinct from every [`Panel::number`].
pub const DOLL_PANEL: u32 = 0x8000_0000;
/// The underwear item, which a displaced piece never returns to the pack as
/// (the pack's give-back skips it).
const BRAIES: u32 = exa_items::carry::BRAIES;

/// The view's rectangle inside the doll panel: the panel less its border
/// on every side (the 3D view's client).
pub fn view_rect(doll: GuiRect, border: i32) -> GuiRect {
    GuiRect::new(
        doll.left + border,
        doll.top + border,
        doll.width() - border * 2,
        doll.height() - border * 2,
    )
}

/// The rating dots' top: the record's top plus half of its height less the
/// dot's, the difference shifted as an unsigned 64-bit value and truncated
/// to 32 bits (a record 2 pixels lower than the dot puts them 1 up, one 4
/// pixels lower 2 up).
pub fn dot_top(y: i32, height: i32, dot: i32) -> i32 {
    y.wrapping_add(((i64::from(height - dot) as u64) >> 1) as i32)
}

/// The view camera's transform: at the character view's eye around the
/// figure's point, both converted from centimetres, looking at the point.
pub fn camera_transform() -> Transform {
    let eye = Vec3::from(default_camera_position()) * 0.01 + DOLL_ORIGIN;
    let target = Vec3::from(FIGURE_POINT) * 0.01 + DOLL_ORIGIN;
    Transform::from_translation(eye).looking_at(target, Vec3::Y)
}

/// The view light: from (0.5, 0.7, 0.5) turned into the camera's space,
/// shining toward the figure.
pub fn light_transform(camera: &Transform) -> Transform {
    let toward = doll_facing(camera.rotation);
    let up = if toward.abs().dot(Vec3::Y) > 0.999 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    Transform::default().looking_to(-toward, up)
}

/// The image the view renders into for a view of `size` pixels.
pub fn image_size(size: (i32, i32)) -> UVec2 {
    image_size_at(size, DOLL_SCALE)
}

/// The image a view of `size` pixels renders into at `scale` pixels a view
/// pixel.
pub fn image_size_at(size: (i32, i32), scale: u32) -> UVec2 {
    let scale = scale.max(1);
    UVec2::new(
        (size.0.max(1) as u32) * scale,
        (size.1.max(1) as u32) * scale,
    )
}

/// The dot sheet's whole and half dots resampled to the interface's large
/// icon cell (the GUI level's, [`dot_side`]) in an atlas of their own.
pub struct DotSheet {
    pub atlas: IconAtlas,
    pub texture: GuiTexture,
    pub whole: i32,
    pub half: i32,
    /// The cell side the dots were cut at.
    pub side: i32,
}

/// The item GUI's dot size at GUI unit `unit`: the interface's large-icon
/// cell, which each GUI level sets (16, 20, 28, 32, 42 and 64 pixels,
/// [`exa_ui::gui_font::LARGE_ICONS`]); the rating dots, the slider's knob
/// and the socket crystals are cut and spaced at it.
pub fn dot_side(unit: i32) -> i32 {
    exa_ui::gui_font::gui_level(exa_ui::gui_font::level_for_unit(unit)).large_icon
}

impl DotSheet {
    /// From the sheet's decoded cells (bottom row first, 64 pixels square,
    /// whole dot first), cut at `side` pixels: `None` unless both draw.
    pub fn of_cells(cells: &[Vec<u8>], cell: usize, side: i32) -> Option<Self> {
        let side = side.max(1) as usize;
        let format = exa_items::item_gui::SHEET_ATLASES[0].format;
        let mut atlas = IconAtlas::new(side as i32, 4);
        let mut handles = Vec::new();
        for (index, pixels) in cells.iter().take(2).enumerate() {
            let resampled = exa_render::mipmap::resample_rgba8_to_size(
                pixels, cell, cell, side, side, format, 0,
            );
            if resampled.len() != side * side * 4 {
                return None;
            }
            let icon = IconImage {
                width: side as u16,
                height: side as u16,
                channels: 4,
                pixels: resampled,
            };
            handles.push(atlas.add(&icon, index as i32 + 1));
        }
        let (&whole, &half) = (handles.first()?, handles.get(1)?);
        if whole == 0 || half == 0 {
            return None;
        }
        let texture = GuiTexture::from_bottom_up(
            u32::from(atlas.image.width),
            u32::from(atlas.image.height),
            &atlas.image.pixels,
        )?;
        Some(Self {
            atlas,
            texture,
            whole,
            half,
            side: side as i32,
        })
    }
}

/// The rating dots of a record at `(x, y)` of height `height`: whole dots
/// up to the value less a quarter, then a half dot, three units apart (two
/// and a half above 7.25), in `ink`. `None` when nothing draws.
pub fn rating_dots_layer(
    sheet: &DotSheet,
    value: f32,
    place: (i32, i32, i32),
    unit: i32,
    ink: [u8; 4],
) -> Option<GuiLayer> {
    let (x, y, height) = place;
    let (dots, half) = exa_items::item_gui::rating_dots(value, x, unit);
    let top = dot_top(y, height, sheet.side) as f32;
    let mut quads = Vec::new();
    for at in dots {
        quads.extend(sheet.atlas.draw(sheet.whole, at as f32, top));
    }
    if let Some(at) = half {
        quads.extend(sheet.atlas.draw(sheet.half, at as f32, top));
    }
    if quads.is_empty() {
        return None;
    }
    Some(GuiLayer::new(
        icon_batch(&quads, ink),
        LayerTexture::Owned(sheet.texture.clone()),
    ))
}

/// What the inventory's painter needs from the view.
pub struct DollPaint<'a> {
    /// The view draws: the canvas is cleared over it.
    pub punch: bool,
    /// The icon slot of the item shown over the figure, 0 for none.
    pub shown_icon: usize,
    /// The rating dots' value, when known.
    pub dots: Option<f32>,
    pub sheet: Option<&'a DotSheet>,
}

/// The view's camera, image, light, node and figure.
pub struct DollScene {
    pub image: Handle<Image>,
    pub camera: Entity,
    pub light: Entity,
    pub node: Entity,
    pub figure: Option<Entity>,
    /// The body the figure copies (the inventory's bound character).
    pub body: Option<Entity>,
    /// The view's rectangle the scene was made for.
    pub view: GuiRect,
    /// The colour the camera clears to.
    pub backdrop: [u8; 4],
    /// The image's pixels a view pixel, taken from the settings when the
    /// scene was made ([`doll_scale`]), as the constructor takes its first
    /// choice of super sampling.
    pub scale: u32,
}

/// The inventory's character view state.
#[derive(Resource, Default)]
pub struct InventoryDoll {
    pub scene: Option<DollScene>,
    /// The rating dots' value: ten times the player's encumbrance.
    pub dots: Option<f32>,
    /// The colour the GUI canvas had over the view before the painter
    /// cleared it, which the camera clears to.
    pub backdrop: [u8; 4],
    pub sheet: Option<DotSheet>,
    /// The dot side the sheet was last tried at.
    sheet_tried: Option<i32>,
    /// The item shown over the figure: the bound body's core crystal, or
    /// one a drop gave it ([`show_on_view`]).
    pub shown: Option<ShownItem>,
    /// The body the view's bind last looked for a shown item on.
    shown_for: Option<Entity>,
}

/// The item a character view shows over its figure, at its lower-left
/// corner: its id, its flags and the icon slot it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShownItem {
    pub id: u32,
    pub flags: u32,
    pub icon: usize,
}

/// Shows an item over the view's figure, as the character view's show
/// does ([`exa_ui::character_view::show_item`]): an item whose icon
/// resolves is remembered with its flags and its icon, anything else (item
/// 0 included) clears the shown item. Exarust releases the icon shown
/// before, which the original's resolve keeps a reference to.
pub fn show_on_view(
    doll: &mut InventoryDoll,
    icons: &mut PanelIcons,
    catalogue: Option<&ItemCatalogue>,
    id: u32,
    flags: u32,
) {
    let previous = doll.shown.map_or(0, |shown| shown.icon);
    let icon = if id == 0 {
        0
    } else {
        icons.reserve(id, flags, catalogue).0
    };
    let shown = exa_ui::character_view::show_item(previous, id, flags, icon);
    if previous != 0 {
        icons.release(previous);
    }
    doll.shown = shown.item.map(|(id, flags)| ShownItem {
        id,
        flags,
        icon: shown.icon,
    });
}

/// The view's bind of a body, for the shown item: the first of the body's
/// worn items that is one of the crystals the view looks for
/// ([`exa_ui::game_gui::crystal_item`]: crystals A, AL, B, C and AR) is
/// shown, else the shown item is cleared. It runs once for each body the
/// view's scene is made for; the shown item goes when the view closes.
fn bind_shown_item(
    mut doll: ResMut<InventoryDoll>,
    icons: Option<ResMut<PanelIcons>>,
    catalogue: Option<Res<ItemCatalogue>>,
    bodies: Query<&Possessions>,
) {
    let Some(mut icons) = icons else {
        return;
    };
    let Some(body) = doll.scene.as_ref().map(|scene| scene.body) else {
        if let Some(shown) = doll.shown.take() {
            icons.release(shown.icon);
        }
        doll.shown_for = None;
        return;
    };
    if doll.shown_for == body && body.is_some() {
        return;
    }
    doll.shown_for = body;
    let worn: Vec<(i32, i32)> = body
        .and_then(|body| bodies.get(body).ok())
        .map(|p| {
            p.worn
                .iter()
                .map(|w| (w.id as i32, w.flags as i32))
                .collect()
        })
        .unwrap_or_default();
    let (id, flags) = exa_ui::game_gui::crystal_item(&worn);
    show_on_view(
        &mut doll,
        &mut icons,
        catalogue.as_deref(),
        id as u32,
        flags as u32,
    );
    if let Some(shown) = doll.shown {
        info!(
            "character view: shows {:#x} (flags {:#x}) over the figure",
            shown.id, shown.flags
        );
    }
}

/// Where the view draws its shown item's icon, from the view's top-left:
/// the icon placed at (unit, view height less unit) by the icon draw's
/// origin rule ([`exa_items::icon_cache::IconCache::draw_origin`]).
pub fn shown_icon_origin(
    icons: &PanelIcons,
    icon: usize,
    unit: i32,
    view_height: i32,
    icon_size: i32,
) -> Option<(i32, i32)> {
    icons
        .cache
        .draw_origin(icon, unit, view_height - unit, icon_size)
}

/// Whether the pointer at `at` (from the view's top-left) is on the shown
/// item's icon: the point taken into the icon's space
/// ([`exa_items::icon_cache::IconCache::test_point`] from (unit, view
/// height less unit)) meets an opaque pixel of it
/// ([`PanelIcons::pixel_hit`]).
pub fn over_shown_icon(
    icons: &PanelIcons,
    icon: usize,
    at: (i32, i32),
    unit: i32,
    view_height: i32,
    icon_size: i32,
) -> bool {
    icons
        .cache
        .test_point(icon, at.0 - unit, at.1 - (view_height - unit), icon_size)
        .is_some_and(|(x, y)| icons.pixel_hit(icon, x, y))
}

/// A drop on the inventory's figure: the drag's source panel and payload.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DollDrop {
    pub source: Panel,
    pub id: u32,
    pub flags: u32,
    pub icon: usize,
    pub bulk: i32,
    /// The release was on the view's right half (a weapon then goes into
    /// the off hand when the character knows Insight's technique 8).
    pub right_half: bool,
}

/// The game's object records by id, decoded for the readers that want an
/// [`Item`]: the base templates, read once when a drop first needs an
/// apparel's occupancy and worn model, and every record of the live
/// database's local and world tiers under its tagged id, refreshed from
/// [`LiveObjects`] whenever it changes ([`Self::sync`]). A kit's copy, a
/// level's own record and a promoted record each read their own bytes.
#[derive(Resource, Default, Debug, Clone)]
pub struct DollItems {
    /// The game's item templates, shared: the fixed step reads a copy of
    /// the set, which must not clone the whole database each time.
    items: std::sync::Arc<HashMap<u32, Item>>,
    /// The live database's tagged records, by their whole id.
    copies: std::sync::Arc<HashMap<u32, Item>>,
    /// The live database's revision the copies were read at.
    revision: u64,
    /// The game's race table, for the view's drag-over test and drop
    /// (`None` until a game is read).
    races: Option<std::sync::Arc<exa_formats::races::RaceDb>>,
    tried: bool,
}

impl DollItems {
    pub(crate) fn ensure(&mut self, game: Option<&Game>) {
        if self.tried {
            return;
        }
        let Some(game) = game else {
            return;
        };
        self.tried = true;
        let items = game
            .0
            .pack(PackKind::Resource)
            .and_then(|p| p.get("objdb.rdb"))
            .and_then(|bytes| exa_formats::rdb::Database::parse(bytes).ok())
            .map(|db| exa_formats::objdb::items(&db))
            .unwrap_or_default();
        self.items = std::sync::Arc::new(items.into_iter().map(|item| (item.id, item)).collect());
        self.races = Some(std::sync::Arc::new(exa_formats::races::RaceDb::new(
            crate::scene_bodies::game_races(game),
        )));
    }

    /// The race of a record's race byte, as the original's race lookup
    /// gives it (its built-in record for an id the table lacks); `None`
    /// before a game is read.
    pub fn race(&self, byte: u8) -> Option<&exa_formats::races::Race> {
        self.races.as_deref().map(|db| db.of_character(byte))
    }

    /// Reads again what the live database changed since the last sync: the
    /// records written in place, or every tagged record after records came
    /// or went.
    pub fn sync(&mut self, live: &LiveObjects) {
        match live.changes_since(self.revision) {
            Changes::None => {}
            Changes::Records(ids) => {
                for id in ids {
                    let Some(item) = live.item(id) else {
                        continue;
                    };
                    if id & !ID_MASK == 0 {
                        std::sync::Arc::make_mut(&mut self.items).insert(id, item);
                    } else {
                        std::sync::Arc::make_mut(&mut self.copies).insert(id, item);
                    }
                }
            }
            Changes::All => {
                self.copies = std::sync::Arc::new(
                    live.tagged_items()
                        .into_iter()
                        .map(|item| (item.id, item))
                        .collect(),
                );
            }
        }
        self.revision = live.revision();
    }

    /// The record `id` names: a tagged id's own record in the live
    /// database, an untagged id's template. A tagged id the live database
    /// does not hold (an arena actor's own database's) reads its template's
    /// [Exarust's fallback: those records live in the actor's own
    /// database].
    pub fn get(&self, id: u32) -> Option<&Item> {
        self.copies
            .get(&id)
            .or_else(|| self.items.get(&(id & ID_MASK)))
    }

    /// Writes a container popup's children back into the item's record in
    /// the live database (a tagged id's own, an untagged id's template's)
    /// and reads it again. An id the database does not hold, or an item
    /// with no container, is left as it was.
    pub fn store_container(
        &mut self,
        live: &mut LiveObjects,
        id: u32,
        record: &crate::item_container::ContainerRecord,
    ) {
        let children: Vec<exa_formats::containers::Child> = record
            .children
            .iter()
            .map(|child| exa_formats::containers::Child {
                id: child.id,
                flags: child.flags,
                x: child.x,
                y: child.y,
            })
            .collect();
        if live.set_container_children(id, &children) {
            self.sync(live);
        }
    }
}

/// The character view's drag-over test for an item dragged onto a figure
/// of `race` ([`exa_ui::character_view::accepts_drag`]): apparel only on a
/// view that allows changes and a race that wears it (the look word's bit
/// 4), a weapon or shield flagged for golems (`0x2000000`) or for the
/// large races (`0x4000000`, weapons only) only on a race whose kind word
/// has the matching bit, and an item with none of those sections only as
/// the AL crystal on a race that takes it while no item is shown. A view
/// the test refuses never gets the drop: the drag ends as a cancelled one.
/// `None` for `race` (no game read) accepts as the human's race does.
pub fn figure_accepts(
    race: Option<&exa_formats::races::Race>,
    id: u32,
    kind: possession::ItemKind,
    shield_flags: Option<u32>,
    has_apparel: bool,
    view_no_changes: bool,
) -> bool {
    figure_accepts_shown(
        race,
        id,
        kind,
        shield_flags,
        has_apparel,
        view_no_changes,
        0,
    )
}

/// [`figure_accepts`] for a view showing the item whose icon slot is
/// `shown_icon` (0 for none): a core crystal is taken only while nothing
/// is shown.
pub fn figure_accepts_shown(
    race: Option<&exa_formats::races::Race>,
    id: u32,
    kind: possession::ItemKind,
    shield_flags: Option<u32>,
    has_apparel: bool,
    view_no_changes: bool,
    shown_icon: usize,
) -> bool {
    let (look, core, body) = race.map_or((4, 0, 0), |r| (r.look, r.flags, r.stats.kind));
    exa_ui::character_view::accepts_drag(exa_ui::character_view::AcceptDragInput {
        refused_by_callback: false,
        id: id & ID_MASK,
        apparel: has_apparel,
        weapon: kind.weapon.is_some(),
        shield: kind.shield,
        section_flags: kind.weapon.or(shield_flags).unwrap_or(0),
        race_flags: look,
        race_core_flags: core,
        body_flags: body,
        view_no_changes,
        shown_icon,
    })
}

/// Marks a mesh of the figure moved onto the view's layer.
#[derive(Component)]
struct DollPart;

/// The view's rectangle on the screen while the inventory is open.
pub fn open_view(screens: &GameScreens) -> Option<GuiRect> {
    let screen = screens.find(exa_ui::game_gui::GameScreen::Inventory)?;
    let bounds = screens.task.form(screen.form)?.bounds();
    let doll = screens.inventory_layout().doll_at(bounds);
    Some(view_rect(doll, gui_screens::DOLL_BORDER))
}

fn despawn_scene(commands: &mut Commands, scene: DollScene) {
    for entity in [scene.camera, scene.light, scene.node]
        .into_iter()
        .chain(scene.figure)
    {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.despawn();
        }
    }
}

fn spawn_scene(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    view: GuiRect,
    backdrop: [u8; 4],
    scale: u32,
) -> DollScene {
    let size = image_size_at((view.width(), view.height()), scale);
    let mut image = Image::new_uninit(
        Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT;
    let image = images.add(image);
    let layer = doll_layers();
    let camera_at = camera_transform();
    let [r, g, b, a] = backdrop;
    let camera = commands
        .spawn((
            Name::new("_InventoryCamera"),
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(Color::srgba_u8(r, g, b, a)),
                ..default()
            },
            RenderTarget::Image(image.clone().into()),
            Projection::Perspective(PerspectiveProjection {
                fov: FIELD_OF_VIEW.to_radians(),
                near: 0.1,
                far: 50.0,
                aspect_ratio: view.width().max(1) as f32 / view.height().max(1) as f32,
                ..default()
            }),
            camera_at,
            Tonemapping::None,
            Msaa::Off,
            layer.clone(),
            exa_render::supersample::Offscreen,
        ))
        .id();
    let light = commands
        .spawn((
            Name::new("_InventoryLight"),
            DirectionalLight {
                color: Color::WHITE,
                illuminance: DOLL_SUN,
                shadow_maps_enabled: false,
                ..default()
            },
            light_transform(&camera_at),
            layer,
            exa_render::supersample::Offscreen,
        ))
        .id();
    let node = commands
        .spawn((
            Name::new("_InventoryView"),
            ImageNode {
                image: image.clone(),
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            GlobalZIndex(DOLL_Z),
        ))
        .id();
    DollScene {
        image,
        camera,
        light,
        node,
        figure: None,
        body: None,
        view,
        backdrop,
        scale,
    }
}

/// The view follows the inventory: made when it opens (and again when the
/// view's size changes), placed over the doll panel, its camera's clear
/// kept at the backdrop, the pointer handed to the figure's turn, and
/// everything dropped when it closes.
#[allow(clippy::too_many_arguments)]
pub(crate) fn sync_doll(
    mut commands: Commands,
    screens: Res<GameScreens>,
    mut doll: ResMut<InventoryDoll>,
    mut images: ResMut<Assets<Image>>,
    mut pointer: ResMut<crate::char_view::PreviewPointer>,
    windows: Query<&Window, With<PrimaryWindow>>,
    players: Query<&exa_combat::Fighter>,
    player_bodies: Query<Entity, With<crate::player::PlayerBody>>,
    mut nodes: Query<&mut Node>,
    mut cameras: Query<&mut Camera>,
    game: Option<Res<Game>>,
    super_sampling: Option<Res<exa_render::supersample::SuperSampling>>,
) {
    let Some(view) = open_view(&screens) else {
        if let Some(scene) = doll.scene.take() {
            despawn_scene(&mut commands, scene);
            pointer.0 = None;
        }
        doll.dots = None;
        return;
    };
    let bound = screens.inventory_bound(player_bodies.iter().next());
    doll.dots = bound
        .and_then(|body| players.get(body).ok())
        .map(|fighter| exa_items::inventory_window::encumbrance_dots(fighter.stats.encumbrance));
    let side = dot_side(screens.pack.unit);
    if doll.sheet_tried != Some(side)
        && let Some(game) = game.as_deref()
    {
        doll.sheet_tried = Some(side);
        doll.sheet = load_dot_sheet(game, side);
    }
    let resized = doll.scene.as_ref().is_some_and(|scene| {
        (scene.view.width(), scene.view.height()) != (view.width(), view.height())
    });
    // A window bound to another character makes the scene again, so the
    // figure is that character's.
    let rebound = doll.scene.as_ref().is_some_and(|scene| scene.body != bound);
    if (resized || rebound)
        && let Some(scene) = doll.scene.take()
    {
        despawn_scene(&mut commands, scene);
    }
    let backdrop = doll.backdrop;
    let sampling = doll_scale(super_sampling.is_none_or(|on| on.0));
    let scene = doll
        .scene
        .get_or_insert_with(|| spawn_scene(&mut commands, &mut images, view, backdrop, sampling));
    scene.view = view;
    scene.body = bound;
    if scene.backdrop != backdrop
        && let Ok(mut camera) = cameras.get_mut(scene.camera)
    {
        let [r, g, b, a] = backdrop;
        camera.clear_color = ClearColorConfig::Custom(Color::srgba_u8(r, g, b, a));
        scene.backdrop = backdrop;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let scale = window.scale_factor();
    if let Ok(mut node) = nodes.get_mut(scene.node) {
        node.left = Val::Px(view.left as f32 / scale);
        node.top = Val::Px(view.top as f32 / scale);
        node.width = Val::Px(view.width() as f32 / scale);
        node.height = Val::Px(view.height() as f32 / scale);
    }
    pointer.0 = window.cursor_position().map(|cursor| {
        let at = cursor * scale - Vec2::new(view.left as f32, view.top as f32);
        crate::char_view::pointer_point(
            &camera_transform(),
            FIELD_OF_VIEW.to_radians(),
            DOLL_ORIGIN,
            at,
            view.width() as f32,
            view.height() as f32,
        )
    });
}

/// Reads the dot sheet and keeps its whole and half dots, cut at `side`.
pub(crate) fn load_dot_sheet(game: &Game, side: i32) -> Option<DotSheet> {
    let sheet = &exa_items::item_gui::SPRITE_SHEETS[0];
    let name = sheet.name.strip_suffix(".rfi").unwrap_or(sheet.name);
    let bytes = game.0.texture(name)?;
    let loaded = exa_items::item_gui::load_sheet(sheet, bytes)?;
    DotSheet::of_cells(&loaded.cells, sheet.cell as usize, side)
}

/// The player's look the figure copies.
type PlayerLook<'a> = (
    Ref<'a, Outfit>,
    &'a exa_assets::BodyModel,
    Option<&'a exa_assets::SkinLook>,
    Option<&'a exa_assets::SkinCompositeJob>,
    Option<&'a exa_assets::BodyMorph>,
    Option<&'a exa_character::hair::Hair>,
    Option<&'a exa_character::RaceWeaponry>,
);

/// Builds the figure from the player's body once the view exists, and
/// keeps its outfit the player's. Its hands are the hand scene's
/// ([`rebuild_doll_hands`]).
#[allow(clippy::too_many_arguments)]
fn dress_doll(
    mut commands: Commands,
    mut doll: ResMut<InventoryDoll>,
    players: Query<PlayerLook, Without<DollHands>>,
    game: Option<Res<Game>>,
    cache: Option<ResMut<ModelCache>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<exa_render::material::ExaMaterial>>>,
    images: Option<ResMut<Assets<Image>>>,
    support: Option<Res<CompressedImageFormatSupport>>,
) {
    let Some(scene) = doll.scene.as_mut() else {
        return;
    };
    let Some((outfit, body, skin, job, morph, hair, weaponry)) =
        scene.body.and_then(|body| players.get(body).ok())
    else {
        return;
    };
    if let Some(figure) = scene.figure {
        if outfit.is_changed() {
            commands.entity(figure).insert(Outfit::clone(&outfit));
        }
        return;
    }
    let (Some(game), Some(mut cache), Some(mut meshes), Some(mut materials), Some(mut images)) =
        (game, cache, meshes, materials, images)
    else {
        return;
    };
    let mut loader = ModelLoader {
        game: &game.0,
        cache: &mut cache,
        meshes: &mut meshes,
        materials: &mut materials,
        images: &mut images,
        bc_supported: model::bc_supported(support.as_deref()),
    };
    let Some(model) = loader.morphed_model(PackKind::Resource, &body.0, morph) else {
        return;
    };
    let figure = exa_world::spawn_model(&mut commands, &model, None);
    commands.entity(figure).insert((
        Name::new("_InventoryFigure"),
        Transform::from_translation(DOLL_ORIGIN),
        Visibility::default(),
        exa_anim::Animator::new("Default", "Idle"),
        exa_anim::FrameClips,
        Outfit::clone(&outfit),
        body.clone(),
        exa_character::WearsUnderwear,
        crate::char_view::PreviewTurn::default(),
        Hands::default(),
        DollHands::default(),
    ));
    if let Some(skin) = skin {
        commands.entity(figure).insert(skin.clone());
    }
    if let Some(job) = job {
        commands.entity(figure).insert(job.clone());
    }
    if let Some(morph) = morph {
        commands.entity(figure).insert(morph.clone());
    }
    if let Some(hair) = hair {
        commands.entity(figure).insert(*hair);
    }
    if let Some(weaponry) = weaponry {
        commands.entity(figure).insert(*weaponry);
    }
    scene.figure = Some(figure);
}

/// The figure's hand scene: the item each hand's model shows (the scene
/// job's "current" hands) and the model's root node.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DollHands {
    pub shown: [Hand; 2],
    pub nodes: [Option<Entity>; 2],
}

/// The hand Exarust's carried torch is in, from the player's hands: the
/// main hand when its item is the torch, else the off hand when it holds a
/// light. `None` without the torch.
pub fn torch_hand(hands: &Hands, carries: bool) -> Option<usize> {
    if !carries {
        return None;
    }
    if hands
        .main
        .is_some_and(|main| main.light && main.flags == crate::torch::TORCH_FLAGS)
    {
        Some(0)
    } else if hands.off_light {
        Some(1)
    } else {
        None
    }
}

/// The hands the figure's scene is built from: the record's out set, with
/// Exarust's carried torch (outside the record, always burning) in its hand
/// when the record leaves that hand empty, else in the other hand when
/// that one is empty.
pub fn figure_targets(out: &[Hand; 2], torch: Option<usize>) -> [Hand; 2] {
    let mut targets = *out;
    if let Some(hand) = torch {
        let torch = Hand {
            id: crate::flow::STORY_TORCH_ITEM,
            flags: possession::LIT,
        };
        if let Some(free) = [hand, 1 - hand].into_iter().find(|&h| targets[h].id == 0) {
            targets[free] = torch;
        }
    }
    targets
}

/// Where a hand model hangs: the bone and its placement under it. A weapon
/// takes its class's grip in the hand's held slot (the first for the main
/// hand, the second for the off hand) with its flip; a light takes the
/// same grip by its own weapon flags (the torch's when it has none),
/// unflipped; a shield the left hand turned
/// [`exa_character::offhand::SHIELD_TURN_DEGREES`] about z (the system then
/// moves it into the forearm's frame at the hand's rest offset, as the
/// original's shield placement has it). A weapon class the grip table
/// lacks hangs at the hand's own bone.
pub fn hand_placement(
    kind: AttachKind,
    hand: usize,
    weapon_flags: Option<u32>,
    flipped: bool,
    holder: &exa_combat::grip::GripHolder,
) -> (usize, Transform) {
    use exa_combat::attack::Slot;
    let own = if hand == 0 {
        exa_character::RIGHT_HAND
    } else {
        exa_character::LEFT_HAND
    };
    let slot = if hand == 0 { Slot::First } else { Slot::Second };
    let gripped = |flags: u32, flipped: bool| {
        exa_combat::grip::grip(flags & 0xFFFF, slot).map_or((own, Transform::IDENTITY), |g| {
            (
                exa_character::grip_bone(g.hand),
                g.placement(holder, flipped),
            )
        })
    };
    match kind {
        AttachKind::Weapon => gripped(weapon_flags.unwrap_or(0), flipped),
        AttachKind::Light => gripped(
            weapon_flags.unwrap_or(u32::from(crate::torch::TORCH_FLAGS)),
            false,
        ),
        AttachKind::Shield => (
            exa_character::LEFT_HAND,
            Transform::from_rotation(Quat::from_rotation_z(
                exa_character::offhand::SHIELD_TURN_DEGREES.to_radians(),
            )),
        ),
    }
}

/// The left forearm's bone, whose frame a shield's placement is in.
const LEFT_FOREARM: usize = 16;

/// The figure's hands the player's record makes.
type PlayerHands<'a> = (
    &'a Possessions,
    Option<&'a Hands>,
    Has<crate::torch::CarriesTorch>,
    Option<&'a exa_combat::Fighter>,
);

/// The figure's own hand scene and what its holder reads.
type FigureHands<'a> = (
    &'a mut DollHands,
    &'a mut Hands,
    Option<&'a mut exa_anim::Animator>,
    Option<&'a exa_character::RaceWeaponry>,
);

/// Runs the view's hand-scene job on the figure whenever the player's
/// record (or the carried torch) changes what its hands hold: the plan by
/// [`scene_plan`], its steps by [`exa_items::doll::AttachPlan::rebuild_steps`]
/// (a hand emptied releases its model by [`release_model`]), the models
/// placed by [`hand_placement`] under the figure's hand bones.
#[allow(clippy::too_many_arguments)]
fn rebuild_doll_hands(
    mut commands: Commands,
    doll: Res<InventoryDoll>,
    players: Query<PlayerHands, Without<DollHands>>,
    mut figures: Query<FigureHands, With<DollHands>>,
    (children, parents, locals): (
        Query<&Children>,
        Query<&ChildOf>,
        Query<&Transform, With<exa_anim::BodyPart>>,
    ),
    (parts, blades, shields): (
        Query<&exa_anim::BodyPart>,
        Query<(), With<exa_combat::Blade>>,
        Query<&exa_combat::block::ShieldHold>,
    ),
    catalogue: Option<Res<ItemCatalogue>>,
    game: Option<Res<Game>>,
    cache: Option<ResMut<ModelCache>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<exa_render::material::ExaMaterial>>>,
    images: Option<ResMut<Assets<Image>>>,
    support: Option<Res<CompressedImageFormatSupport>>,
) {
    let Some(figure) = doll.scene.as_ref().and_then(|scene| scene.figure) else {
        return;
    };
    let (Some(catalogue), Some(game)) = (catalogue.as_deref(), game) else {
        return;
    };
    let Some((possessions, player_hands, carries, fighter)) = doll
        .scene
        .as_ref()
        .and_then(|scene| scene.body)
        .and_then(|body| players.get(body).ok())
    else {
        return;
    };
    let Ok((mut scene, mut hands, mut animator, race)) = figures.get_mut(figure) else {
        return;
    };
    let torch = player_hands.and_then(|h| torch_hand(h, carries));
    let targets = figure_targets(&possessions.out, torch);
    let kind = |id: u32| catalogue.kind(id & ID_MASK);
    let modelled = |id: u32| catalogue.model(id & ID_MASK).is_some();
    let Some(plan) = scene_plan(&scene.shown, &targets, kind, modelled, false) else {
        return;
    };
    // The hand bones must exist before the job runs.
    let by_bone: HashMap<usize, Entity> = children
        .iter_descendants(figure)
        .filter_map(|e| parts.get(e).ok().map(|p| (p.bone, e)))
        .collect();
    if by_bone.is_empty() {
        return;
    }
    let (Some(mut cache), Some(mut meshes), Some(mut materials), Some(mut images)) =
        (cache, meshes, materials, images)
    else {
        return;
    };
    let mut loader = ModelLoader {
        game: &game.0,
        cache: &mut cache,
        meshes: &mut meshes,
        materials: &mut materials,
        images: &mut images,
        bc_supported: model::bc_supported(support.as_deref()),
    };
    let size = fighter.map_or(1.0, |f| f.stats.size);
    let holder = race.copied().unwrap_or_default().grip_holder(size);
    let present = [
        scene.nodes[0].map(|_| scene.shown[0].id),
        scene.nodes[1].map(|_| scene.shown[1].id),
    ];
    let main_grip = kind(targets[0].id).map(|k| k.grip());
    let both_empty = targets[0].id == 0 && targets[1].id == 0;
    let steps = plan.rebuild_steps(&present, class_word(main_grip), 0);
    info!(
        "doll hands: {:#x}/{:#x} -> {:#x}/{:#x}, steps {steps:?}",
        scene.shown[0].id, scene.shown[1].id, targets[0].id, targets[1].id
    );
    for step in steps {
        match step {
            ApplyStep::Detach(hand) => {
                // A hand left empty is the drag-off's take-out; a hand
                // that takes another item makes room with a plain one.
                let mode = if targets[hand].id == 0 {
                    TakeOut::scheduled(hand).mode
                } else {
                    TakeOut::plain(hand, targets[hand].id).mode
                };
                let release = release_model(both_empty, mode);
                if release.detach_node
                    && let Some(node) = scene.nodes[hand].take()
                    && let Ok(mut e) = commands.get_entity(node)
                {
                    e.despawn();
                }
                if release.restart
                    && let Some(anim) = animator.as_mut()
                {
                    anim.start("Idle", true, None, STANCE_BLEND, exa_anim::Ease::Linear);
                }
                debug!("doll hands: hand {hand} released ({release:?})");
            }
            ApplyStep::Attach { hand, kind: attach } => {
                let Some(take) = plan.attach[hand] else {
                    continue;
                };
                let Some(item) = catalogue.model(take.id & ID_MASK) else {
                    continue;
                };
                let loaded = match item.model.strip_prefix('@') {
                    Some(factory) => loader.factory_weapon(factory, item.weapon.as_ref()),
                    None => loader
                        .model(PackKind::Objects, &item.model)
                        .or_else(|| loader.model(PackKind::Objlib, &item.model)),
                };
                let Some(loaded) = loaded else {
                    warn!("doll hands: {} not found", item.model);
                    continue;
                };
                let sections = kind(take.id).unwrap_or_default();
                let marks = exa_items::doll::model_marks(take.flags);
                let (bone, mut placement) =
                    hand_placement(attach, hand, sections.weapon, marks.flipped, &holder);
                let Some(&(mut bone_entity)) = by_bone.get(&bone) else {
                    continue;
                };
                let mut frame = bone;
                // A shield sits at the hand's rest offset in the forearm's
                // frame (the hand's own turn left out), when the hand hangs
                // under its forearm.
                if attach == AttachKind::Shield
                    && let Some(&forearm) = by_bone.get(&LEFT_FOREARM)
                    && parents
                        .get(bone_entity)
                        .is_ok_and(|p| p.parent() == forearm)
                    && let Ok(rest) = locals.get(bone_entity)
                {
                    placement.translation = rest.translation;
                    bone_entity = forearm;
                    frame = LEFT_FOREARM;
                }
                let (root, _) =
                    exa_world::spawn_model_nodes(&mut commands, &loaded, Some(placement));
                commands.entity(root).insert((
                    Name::new(format!("_InventoryHand{hand}")),
                    ChildOf(bone_entity),
                ));
                // The figure pick finds a held model's slot by its striker,
                // the figure's shield or this marker: the hand roots built
                // here (a weapon, a torch, an off-hand item) carry neither of the
                // first two, so they take the marker.
                let shield = shields.get(figure).ok().map(|hold| hold.item);
                if let Some(mark) =
                    figure_held_mark(hand, blades.contains(root), Some(root) == shield)
                {
                    commands.entity(root).insert(mark);
                }
                if let Some(old) = scene.nodes[hand].replace(root)
                    && let Ok(mut e) = commands.get_entity(old)
                {
                    e.despawn();
                }
                info!(
                    "doll hands: hand {hand} shows {:#x} ({}) as {attach:?} under bone {frame}, node {:?}, flipped {}, lit {}",
                    take.id,
                    item.model,
                    hand_node(sections),
                    marks.flipped,
                    marks.lit
                );
            }
            ApplyStep::RefreshGrip => {
                let held = Possessions {
                    out: targets,
                    ..Possessions::default()
                };
                hands.set_if_neq(crate::equip::hands_of(&held, kind));
            }
            ApplyStep::RestartStance => {
                if let Some(anim) = animator.as_mut() {
                    anim.start("Idle", true, None, STANCE_BLEND, exa_anim::Ease::Linear);
                }
            }
            ApplyStep::ClearWeaponPart
            | ApplyStep::RefreshBody
            | ApplyStep::ClearAttachFlag
            | ApplyStep::ExchangeSets => {}
        }
    }
    // A changed hand whose old model the steps did not detach (its new item
    // has no model) keeps nothing.
    for hand in 0..2 {
        if plan.changed[hand]
            && plan.attach[hand].is_none()
            && let Some(node) = scene.nodes[hand].take()
            && let Ok(mut e) = commands.get_entity(node)
        {
            e.despawn();
        }
    }
    scene.shown = targets;
}

/// The figure pick's marker for the model root a figure hand shows
/// (`hand` 0 the main hand, 1 the off hand): its 1-based item slot, unless
/// the root already answers the pick as a striker (it carries a
/// [`exa_combat::Blade`]) or as the figure's shield
/// ([`exa_combat::block::ShieldHold::item`]).
pub fn figure_held_mark(
    hand: usize,
    carries_blade: bool,
    is_shield: bool,
) -> Option<crate::figure_pick::FigureHeld> {
    (!carries_blade && !is_shield).then_some(crate::figure_pick::FigureHeld(hand + 1))
}

/// Moves every mesh of the figure onto the view's layer, out of the shadow
/// links (which would put a caster back on the world's layer).
fn tag_doll_parts(
    mut commands: Commands,
    doll: Res<InventoryDoll>,
    children: Query<&Children>,
    meshes: Query<(), (With<Mesh3d>, Without<DollPart>)>,
) {
    let Some(figure) = doll.scene.as_ref().and_then(|scene| scene.figure) else {
        return;
    };
    let layer = doll_layers();
    for entity in std::iter::once(figure).chain(children.iter_descendants(figure)) {
        if meshes.contains(entity) {
            commands.entity(entity).insert((
                layer.clone(),
                NotShadowCaster,
                NotShadowReceiver,
                DollPart,
            ));
        }
    }
}

/// The weapon class word the take-in plan reads, from the main hand: a
/// one-handed weapon's 0x10, a two-handed one's 0x20, else the fists'
/// 0x80. [guess: the body's class word is not kept by Exarust]
pub fn class_word(main_grip: Option<u32>) -> u32 {
    match main_grip {
        Some(possession::ONE_HANDED) => 0x10,
        Some(0x20) | Some(0x30) => 0x20,
        _ => 0x80,
    }
}

/// The garment an apparel record is worn as.
pub(crate) fn garment_of(item: &Item) -> Option<Garment> {
    (!item.worn_model.is_empty()).then(|| Garment {
        model: item.worn_model.clone(),
        appearance: item.appearance,
        hides: Some(item.hidden_slots()),
        armour: item.armour,
    })
}

/// Takes an item into the out set by the take-in plan: the hands it frees
/// first (their carried copies lose the held mark), a light moving from
/// the main hand to the off hand, then the item into its hand. `off_wanted`
/// is the character-view near hand (the pointer's right half with Insight's
/// technique 8). `kind_of` gives a held item's sections. The record entry
/// takes the attach job's flags, which keep only the top bit.
pub fn hold_item(
    p: &mut Possessions,
    id: u32,
    flags: u32,
    kind: possession::ItemKind,
    off_wanted: bool,
    kind_of: impl Fn(u32) -> Option<possession::ItemKind>,
) {
    let main_kind = (p.out[0].id != 0).then(|| kind_of(p.out[0].id).unwrap_or_default());
    let main = main_kind.map(|k| (p.out[0].id, k.light));
    let off = (p.out[1].id != 0).then_some(p.out[1].id);
    let into = take_into_hands(
        IntoFacts {
            kind,
            main,
            off,
            class: class_word(main_kind.map(|k| k.grip())),
            off_wanted,
        },
        flags,
    );
    for step in &into.plan.steps {
        match *step {
            HoldStep::Release(hand) => {
                let old = p.out[hand];
                possession::release_held(&mut p.carried, old.id, 1);
                p.out[hand] = Hand::default();
            }
            HoldStep::LightToOff => {
                if let Some(light) = into.moved_light {
                    p.out[1] = light;
                    p.out[0] = Hand::default();
                }
            }
        }
    }
    record_into(p, &into.plan, id, into.flags);
}

/// The held part's mark a weapon or light take-in leaves: lit when the
/// dragged item's own flags have [`possession::LIT`], whatever the record
/// keeps.
pub fn held_mark(id: u32, flags: u32) -> possession::HeldMark {
    possession::HeldMark {
        item: id,
        lit: flags & possession::LIT != 0,
    }
}

/// Takes a shield into the off hand by the shield take
/// ([`exa_items::doll::take_shield`]): the off hand's item is taken out
/// first (its carried copy loses the held mark), the record's off hand takes
/// the shield with all its flags. The answer's `clears_part` (no weapon
/// class bits in the class word guessed from the main hand's grip) is the
/// world body's return to empty-hand words, which Exarust's hands refresh
/// writes after the hands change (`exa_character::body_words`); the figure
/// has no physics.
pub fn hold_shield(
    p: &mut Possessions,
    id: u32,
    flags: u32,
    kind_of: impl Fn(u32) -> Option<possession::ItemKind>,
) -> exa_items::doll::ShieldTaken {
    let old = p.out[1];
    if old.id != 0 {
        possession::release_held(&mut p.carried, old.id, 1);
    }
    let main_kind = (p.out[0].id != 0).then(|| kind_of(p.out[0].id).unwrap_or_default());
    let off = (old.id != 0).then_some((old.id, exa_items::doll::HeldPlace::Shield));
    exa_items::doll::take_shield(off, id, flags, p, class_word(main_kind.map(|k| k.grip())))
}

/// The inventory's give-back of an item a take-in freed from a hand: the
/// open pack shows its held entry of the item again
/// ([`crate::item_container::take_back_held`]); a pack that does not list
/// it takes it by its centre add and place pass, and the carried list
/// gains it at that cell. Without an open pack the carried list takes it
/// at the give-back place ([`Possessions::give_back`]) unless it already
/// lists it. The carried copy's held mark was cleared by the hand's
/// release.
pub fn give_back_displaced(
    screens: &mut GameScreens,
    icons: &mut PanelIcons,
    catalogue: Option<&ItemCatalogue>,
    p: &mut Possessions,
    old: Hand,
    roll: &mut u32,
) {
    let listed = p.carried.iter().any(|c| c.id == old.id);
    let pack_open = crate::panel_live::open_panels(screens)
        .iter()
        .any(|(panel, _)| *panel == Panel::PlayerPack);
    if !pack_open {
        if !listed {
            p.give_back(old.id, old.flags & !HELD);
        }
        return;
    }
    if crate::item_container::take_back_held(&mut screens.pack.entries, old.id) {
        debug!("doll drop: {:#x} comes back to the pack's view", old.id);
        return;
    }
    if screens.pack.entries.iter().any(|e| e.id == old.id) {
        return;
    }
    let flags = old.flags & !HELD;
    let (icon, _) = icons.reserve(old.id, flags, catalogue);
    let bulk = catalogue.and_then(|c| c.bulk(old.id)).map_or(0, i32::from);
    if let Some(cell) = icons.add_placed(&mut screens.pack, old.id, icon, flags, bulk, roll)
        && !listed
    {
        let (unit, vscale) = (screens.pack.unit, screens.pack.vscale);
        body_inventory::give(p, old.id, flags, cell, unit, vscale);
    }
}

/// Runs the character view's drop for the drops the panel input handed
/// over: the drop's plan by the item's sections, then the source panel's
/// drag end (the pack's linked view keeps its bulk), the record and the
/// player's outfit, hands and held weapon.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn equip_dropped(
    mut commands: Commands,
    mut drops: MessageReader<DollDrop>,
    mut screens: ResMut<GameScreens>,
    (mut icons, mut roll): (
        ResMut<PanelIcons>,
        ResMut<crate::thaum::live::FrameEffectRoll>,
    ),
    catalogue: Option<Res<ItemCatalogue>>,
    mut items: ResMut<DollItems>,
    game: Option<Res<Game>>,
    mut bodies: Query<
        (
            Entity,
            &mut Possessions,
            Option<&mut Outfit>,
            Option<&mut Hands>,
            Has<crate::torch::CarriesTorch>,
            Option<&exa_character::record::CharacterData>,
            Option<&exa_character::held::HeldMoveset>,
        ),
        Without<DollHands>,
    >,
    player_bodies: Query<Entity, With<crate::player::PlayerBody>>,
    (mut trust, live): (crate::view_trust::ViewTrust, Option<Res<LiveObjects>>),
    mut doll: Option<ResMut<InventoryDoll>>,
) {
    // The character the inventory is bound to: its figure takes the drop.
    let target = screens.inventory_bound(player_bodies.iter().next());
    for drop in drops.read().copied() {
        items.ensure(game.as_deref());
        let catalogue = catalogue.as_deref();
        let kind = catalogue.and_then(|c| c.kind(drop.id)).unwrap_or_default();
        let record = items.get(drop.id).cloned();
        let has_apparel = record
            .as_ref()
            .is_some_and(|item| item.burden.is_some() && !item.worn_model.is_empty());
        let race = target
            .and_then(|t| bodies.get(t).ok())
            .and_then(|entry| entry.5)
            .and_then(|data| items.race(data.record.race));
        let shown_icon = doll
            .as_deref()
            .and_then(|doll| doll.shown)
            .map_or(0, |shown| shown.icon);
        if !figure_accepts_shown(
            race,
            drop.id,
            kind,
            catalogue.and_then(|c| c.shield_flags(drop.id)),
            has_apparel,
            false,
            shown_icon,
        ) {
            info!(
                "character view: refuses the drag of {:#x} (race {:?})",
                drop.id,
                race.map(|r| r.name.as_str())
            );
            cancel(&mut screens, drop);
            continue;
        }
        let race_wears_apparel = race.is_none_or(|r| r.look & 4 != 0);
        let insight = drop.right_half
            && target
                .and_then(|t| bodies.get(t).ok())
                .is_some_and(|entry| {
                    entry.5.is_some_and(|data| {
                        let skill = &data.record.skills[exa_formats::characters::INSIGHT];
                        exa_ui::character_view::insight_known(skill.points, &skill.techniques)
                    })
                });
        // With the mind window set the drop weighs the change: the
        // apparel's wear room over what the body wears, or the weapon's
        // desirability and what its take-in displaces from the hands.
        let mind_cost = screens.inventory_mind_window;
        let (apparel_cost, weapon_value, displaced) = match target.and_then(|t| bodies.get(t).ok())
        {
            Some(entry) if mind_cost => {
                let p = entry.1;
                let values = crate::view_trust::ItemValues::of(live.as_deref(), drop.id);
                let apparel_cost = values
                    .apparel
                    .as_ref()
                    .map_or(0.0, |piece| wear_room_over(live.as_deref(), &p.worn, piece));
                let displaced = displaced_by(
                    p,
                    kind,
                    insight,
                    entry.6.map(|held| u32::from(held.weapon_class)),
                    |id| {
                        crate::view_trust::held_value(&crate::view_trust::ItemValues::of(
                            live.as_deref(),
                            id,
                        ))
                    },
                    |id| catalogue.and_then(|c| c.kind(id)),
                );
                (apparel_cost, values.desirability(), displaced)
            }
            _ => (0.0, 0.0, 0.0),
        };
        let plan = figure_drop(DropFacts {
            id: drop.id,
            flags: drop.flags,
            from_panel: drop.source == Panel::PlayerPack,
            shown: true,
            other_panel: true,
            mind_cost,
            apparel_cost,
            weapon_value,
            displaced,
            has_apparel,
            has_weapon: kind.weapon.is_some(),
            has_shield: kind.shield,
            race_wears_apparel,
            insight,
            // The body takes the item as its core crystal: its race's core
            // word names the item and it wears none of the crystals.
            takes_core: race.is_some_and(|r| {
                let worn: Vec<u32> = target
                    .and_then(|t| bodies.get(t).ok())
                    .map(|entry| entry.1.worn.iter().map(|w| w.id).collect())
                    .unwrap_or_default();
                possession::takes_core(r.flags, drop.id & ID_MASK, &worn)
            }),
            ..DropFacts::default()
        });
        // The change goes into the bound body's trust before the item is
        // taken; a binding that ends closes the window and the drop does
        // nothing.
        if let (Some(cost), Some(body)) = (plan.cost, target)
            && trust.change(body, cost)
        {
            info!(
                "character view: the drop of {:#x} ends the shown body's binding; \
                 the inventory closes",
                drop.id
            );
            cancel(&mut screens, drop);
            screens.close(exa_ui::game_gui::GameScreen::Inventory);
            continue;
        }
        let Some(worn_now) = target
            .and_then(|t| bodies.get(t).ok())
            .map(|entry| entry.1.worn.clone())
        else {
            cancel(&mut screens, drop);
            continue;
        };
        // The apparel's conflict pass may refuse it (a locked piece in the
        // way); nothing moves then.
        let conflicts = match (plan.kind, record.as_ref()) {
            (FigureDrop::Apparel, Some(item)) => {
                let mut data = exa_character::record::CharacterData::of_record(
                    exa_formats::characters::CharacterRecord {
                        worn: worn_now,
                        ..Default::default()
                    },
                );
                match data.equip_conflicts(item, |id| items.get(id)) {
                    Some(removed) => Some((removed, data.record.worn)),
                    None => {
                        cancel(&mut screens, drop);
                        continue;
                    }
                }
            }
            (FigureDrop::Weapon | FigureDrop::Shield | FigureDrop::Hand, _) => None,
            _ => {
                cancel(&mut screens, drop);
                continue;
            }
        };
        // The source's drag ends. A weapon or shield from the view's own
        // pack stays in it at its cell, marked held (the original's hold
        // of the dragged entry, in place of the source's drag end); from
        // any other panel the drag ends: the inventory's pack is linked to
        // the view and keeps the item's bulk; another panel loses it.
        let into_hand = matches!(plan.kind, FigureDrop::Weapon | FigureDrop::Shield);
        if into_hand && drop.source == Panel::PlayerPack {
            if let Some(at) = crate::item_container::hold_dragged(&mut screens.pack.entries) {
                debug!("doll drop: pack entry {at} ({:#x}) is held", drop.id);
            }
        } else if let Some(container) = crate::panel_live::container_mut(&mut screens, drop.source)
        {
            container.finish_drag(DOLL_PANEL, false, true, 0, drop.bulk);
        }
        // The record side: the item leaves its source's list (an item
        // from the player's own pack stays listed when it goes into a
        // hand, marked held).
        let flags = match drop.source {
            Panel::PlayerPack if into_hand => Some(drop.flags & !HELD),
            Panel::PlayerPack => target
                .and_then(|t| bodies.get_mut(t).ok())
                .and_then(|mut b| body_inventory::take(&mut b.1, PanelKind::Pack, drop.id)),
            Panel::BodyPack(body) => bodies
                .iter_mut()
                .find(|entry| entry.0.index().index() as i32 == body)
                .and_then(|mut b| body_inventory::take(&mut b.1, PanelKind::Pack, drop.id)),
            Panel::BodyEquipment(_) | Panel::Figure | Panel::Socket(_) => None,
            Panel::Container(_) => Some(drop.flags & !HELD),
        };
        let Some(flags) = flags else {
            continue;
        };
        let Some((player, mut p, mut outfit, mut hands, torch, _, _)) =
            target.and_then(|t| bodies.get_mut(t).ok())
        else {
            continue;
        };
        if plan.charge_panel {
            screens.pack.used += drop.bulk;
        }
        let pack_unit = screens.pack.unit;
        let pack_vscale = screens.pack.vscale;
        let centre = |screens: &GameScreens| {
            let half = screens.pack.icon_size >> 1;
            Cell::new(
                (screens.pack.client.0 >> 1) - half,
                (screens.pack.client.1 >> 1) - half,
            )
        };
        match plan.kind {
            // The core crystal: the view shows it over the figure; it has
            // left its source, and the record's lists do not take it.
            FigureDrop::Hand => {
                if let Some(doll) = doll.as_deref_mut() {
                    show_on_view(doll, &mut icons, catalogue, drop.id, flags);
                }
                // The world body the view stands for loses its coreless
                // mark (no armour pass runs: the crystal is not worn).
                if plan.clear_mark {
                    commands
                        .queue(move |world: &mut World| crate::equip::core_shown(world, player));
                }
                info!(
                    "character view: the core crystal {:#x} is shown over {:?}'s figure",
                    drop.id, player
                );
            }
            FigureDrop::Apparel => {
                let Some((removed, worn)) = conflicts else {
                    continue;
                };
                // The displaced pieces go back to the pack (the braies are
                // dropped), and the outfit loses their garments.
                for piece in &removed {
                    if let (Some(item), Some(outfit)) = (items.get(piece.id), outfit.as_mut()) {
                        outfit.0.retain(|garment| {
                            !garment.model.eq_ignore_ascii_case(&item.worn_model)
                        });
                    }
                    if piece.id == BRAIES {
                        continue;
                    }
                    let (icon, _) = icons.reserve(piece.id, piece.flags, catalogue);
                    let bulk = catalogue
                        .and_then(|c| c.bulk(piece.id))
                        .map_or(0, i32::from);
                    // The centre add and its place pass: the piece's cell
                    // is where the pass put it.
                    let cell = icons
                        .add_placed(
                            &mut screens.pack,
                            piece.id,
                            icon,
                            piece.flags,
                            bulk,
                            roll.word(),
                        )
                        .unwrap_or_else(|| centre(&screens));
                    body_inventory::give(
                        &mut p,
                        piece.id,
                        piece.flags,
                        cell,
                        pack_unit,
                        pack_vscale,
                    );
                }
                let mut worn = worn;
                worn.push(ItemRef::new(drop.id, flags));
                p.worn = worn;
                if let (Some(garment), Some(outfit)) =
                    (record.as_ref().and_then(garment_of), outfit.as_mut())
                {
                    outfit.0.push(garment);
                }
            }
            FigureDrop::Weapon | FigureDrop::Shield => {
                let before = [p.out[0], p.out[1]];
                if plan.kind == FigureDrop::Weapon {
                    hold_item(&mut p, drop.id, flags, kind, plan.off_wanted, |id| {
                        catalogue.and_then(|c| c.kind(id))
                    });
                    // The weapon or light attach lights the held part from
                    // the item's full flags, while the record keeps only
                    // the job's top bit: the part's mark is what the light
                    // toggle reads.
                    if p.first_held() == drop.id {
                        let mark = held_mark(drop.id, flags);
                        info!(
                            "doll drop: {:#x} held, record flags {:#x}, part lit {}",
                            drop.id, p.out[0].flags, mark.lit
                        );
                        commands.entity(player).insert(mark);
                    }
                } else {
                    hold_shield(&mut p, drop.id, flags, |id| {
                        catalogue.and_then(|c| c.kind(id))
                    });
                }
                // The hands the take-in freed give their items back to the
                // pack ([`give_back_displaced`]).
                for old in before {
                    if old.id != 0 && old.id != drop.id && !p.out.iter().any(|h| h.id == old.id) {
                        give_back_displaced(
                            &mut screens,
                            &mut icons,
                            catalogue,
                            &mut p,
                            old,
                            roll.word(),
                        );
                    }
                }
                if drop.source == Panel::PlayerPack {
                    possession::mark_held(&mut p.carried, drop.id, 1);
                } else {
                    // The view's pack takes the item marked held: its
                    // centre add and place pass, the record at that cell.
                    let cell = if plan.give_to_panel {
                        icons
                            .add_placed(
                                &mut screens.pack,
                                drop.id,
                                drop.icon,
                                flags | HELD,
                                drop.bulk,
                                roll.word(),
                            )
                            .unwrap_or_else(|| centre(&screens))
                    } else {
                        centre(&screens)
                    };
                    body_inventory::give(
                        &mut p,
                        drop.id,
                        flags | HELD,
                        cell,
                        pack_unit,
                        pack_vscale,
                    );
                }
                let mut next = crate::equip::hands_of(&p, |id| catalogue.and_then(|c| c.kind(id)));
                if let Some(hands) = hands.as_mut() {
                    if torch {
                        next = crate::equip::keep_torch(next, hands);
                    }
                    hands.set_if_neq(next);
                }
                let main = p.out[0].id;
                match catalogue.and_then(|c| c.model(main)).filter(|_| main != 0) {
                    Some(model) => {
                        commands.entity(player).insert(Wielded {
                            model: model.model.clone(),
                            weapon: model.weapon,
                        });
                    }
                    None => {
                        commands.entity(player).remove::<Wielded>();
                    }
                }
            }
            _ => {}
        }
    }
}

/// One drop of the debug doll script: the item, its flags, the frame after
/// the load, and whether it lands on the view's right half with Insight's
/// technique 8 known (the near hand, which takes a second weapon).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DollDropEntry {
    pub id: u32,
    pub flags: u32,
    pub frame: u32,
    pub near: bool,
}

/// `--doll-drop ID:FLAGS@FRAME[+],...`: each item, with its flags, put into
/// the player's pack and dropped onto the character view at its frame
/// after the load, as a drag from the pack onto the doll would (a debug
/// tool). A `+` drops it on the view's right half and first writes
/// Insight's technique 8 into the player's record, so a weapon goes into
/// the off hand by the drop's own rule.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct DollDropScript(pub Vec<DollDropEntry>);

impl DollDropScript {
    pub fn parse(text: &str) -> Option<Self> {
        text.split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(|entry| {
                let (entry, near) = match entry.strip_suffix('+') {
                    Some(rest) => (rest, true),
                    None => (entry, false),
                };
                let (item, frame) = entry.split_once('@')?;
                let (id, flags) = item.split_once(':').unwrap_or((item, "0"));
                Some(DollDropEntry {
                    id: crate::gui_drop::parse_id(id)?,
                    flags: crate::gui_drop::parse_id(flags)?,
                    frame: frame.trim().parse().ok()?,
                    near,
                })
            })
            .collect::<Option<Vec<_>>>()
            .filter(|entries| !entries.is_empty())
            .map(Self)
    }
}

/// The debug doll drops ([`DollDropScript`]).
fn scripted_doll_drop(
    script: Res<DollDropScript>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut frames: Local<u32>,
    mut players: Query<
        (
            &mut Possessions,
            Option<&mut exa_character::record::CharacterData>,
        ),
        With<crate::player::PlayerBody>,
    >,
    mut drops: MessageWriter<DollDrop>,
) {
    let Some(last) = script.0.iter().map(|e| e.frame).max() else {
        return;
    };
    if loading.is_some() || *frames > last {
        return;
    }
    *frames += 1;
    for entry in script.0.iter().filter(|e| e.frame == *frames) {
        let Some((mut p, data)) = players.iter_mut().next() else {
            return;
        };
        if entry.near
            && let Some(mut data) = data
        {
            // Insight's points reach one technique, and it is the eighth.
            let skill = &mut data.record.skills[exa_formats::characters::INSIGHT];
            skill.points = skill.points.max(5000);
            if let Some(first) = skill.techniques.first_mut() {
                *first = 8;
            }
        }
        p.carried.push(possession::Carried {
            id: entry.id,
            flags: entry.flags,
            x: 0,
            y: 0,
        });
        info!(
            "doll drop script: {:#x} (flags {:#x}) onto the doll at frame {}{}",
            entry.id,
            entry.flags,
            entry.frame,
            if entry.near { ", near hand" } else { "" }
        );
        drops.write(DollDrop {
            source: Panel::PlayerPack,
            id: entry.id,
            flags: entry.flags,
            icon: 0,
            bulk: 0,
            right_half: entry.near,
        });
    }
}

/// The wear room a piece of apparel adds over what a body wears
/// ([`exa_ai::search::wear_room`]), its pieces read from the live
/// database's records.
pub fn wear_room_over(
    live: Option<&LiveObjects>,
    worn: &[ItemRef],
    piece: &exa_ai::search::Apparel,
) -> f32 {
    let items: std::collections::BTreeMap<u32, exa_ai::search::Loot> = worn
        .iter()
        .filter_map(|w| {
            let values = crate::view_trust::ItemValues::of(live, w.id);
            Some((
                w.id,
                exa_ai::search::Loot::Apparel(Box::new(values.apparel?)),
            ))
        })
        .collect();
    let pack = exa_ai::search::Pack {
        carried: Vec::new(),
        worn: worn.iter().map(|w| w.id).collect(),
        hands: [0; 4],
        locked: false,
    };
    exa_ai::search::wear_room(&items, &pack, piece)
}

/// What a weapon's take-in into the record's hands displaces, weighed
/// ([`possession::displaced_value`]) by the same inputs as [`hold_item`]:
/// the main hand's item (and whether it is a light), the off hand's, the
/// body's weapon class word `class` (what its held items made of it,
/// [`exa_character::held::HeldMoveset`]; guessed from the main hand's
/// grip for a body without one, [`class_word`]), and the near hand.
pub fn displaced_by(
    p: &Possessions,
    kind: possession::ItemKind,
    off_wanted: bool,
    class: Option<u32>,
    value: impl Fn(u32) -> f32,
    kind_of: impl Fn(u32) -> Option<possession::ItemKind>,
) -> f32 {
    let main_kind = (p.out[0].id != 0).then(|| kind_of(p.out[0].id).unwrap_or_default());
    let main = main_kind.map(|k| possession::Holding {
        id: p.out[0].id,
        light: k.light,
    });
    let off = (p.out[1].id != 0).then_some(possession::Holding {
        id: p.out[1].id,
        light: false,
    });
    possession::displaced_value(
        kind.grip(),
        kind.light,
        main,
        off,
        class.unwrap_or_else(|| class_word(main_kind.map(|k| k.grip()))),
        off_wanted,
        value,
    )
}

/// A refused drop: the source's drag is cancelled and its entry stays.
fn cancel(screens: &mut GameScreens, drop: DollDrop) {
    if let Some(container) = crate::panel_live::container_mut(screens, drop.source) {
        container.finish_drag(DOLL_PANEL, true, true, 0, drop.bulk);
    }
}

/// Keeps [`DollItems`] on the live database's records.
fn sync_doll_items(mut items: ResMut<DollItems>, live: Res<LiveObjects>) {
    // Touched only when the live database moved on, so the item set (which
    // the fixed step reads as a copy) is not sent every frame.
    if items.revision != live.revision() {
        items.sync(&live);
    }
}

/// The inventory's character view.
pub struct DollViewPlugin;

impl Plugin for DollViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryDoll>()
            .init_resource::<crate::thaum::live::FrameEffectRoll>()
            .init_resource::<DollItems>()
            .init_resource::<DollDropScript>()
            .add_message::<DollDrop>()
            .init_resource::<LiveObjects>()
            .add_systems(
                Update,
                sync_doll_items
                    .before(gui_screens::draw_screens)
                    .before(crate::panel_live::track_pointer)
                    .in_set(exa_physics::FramePhase::Gui),
            )
            .add_systems(
                Update,
                (
                    sync_doll
                        .after(crate::dialogue_live::paint_dialogue)
                        .after(crate::skill_screen::draw)
                        .after(crate::char_view::turn_previews),
                    bind_shown_item.after(crate::arsenal_view::open_arsenal),
                    dress_doll.after(crate::hw_config::apply_options),
                    scripted_doll_drop,
                    equip_dropped,
                    rebuild_doll_hands.after(crate::creator_view::build_figure),
                )
                    .chain()
                    .after(gui_screens::draw_screens)
                    .in_set(exa_physics::FramePhase::Gui),
            )
            .add_systems(
                PostUpdate,
                tag_doll_parts
                    .before(exa_world::shadow_links::LightCells)
                    .in_set(exa_physics::FramePostPhase::Prepare),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weapon_item(id: u32) -> Item {
        Item {
            id,
            name: "blade".into(),
            parent: 0,
            model: "BladeA01".into(),
            model_flags: 0,
            worn_model: String::new(),
            appearance: None,
            slot_layers: [0; exa_formats::objdb::SLOTS],
            slot_words: [0; exa_formats::objdb::SLOTS],
            occupancy: [0; exa_formats::objdb::OCCUPANCY_BONES],
            weapon: Some(exa_formats::objdb::Weapon {
                dirt: 0x80,
                damage: 0x60,
                ..Default::default()
            }),
            armour: None,
            shield: None,
            door: None,
            operative: None,
            burden: None,
            equip_lock: 0,
            torch: false,
            volume: None,
            volume_script: None,
            use_script: None,
            rank_word: 0,
            body_rank: 0,
            loadout_rank: 0,
            armour_weight: 0.0,
            armour_type: 0,
            container: Some(exa_formats::containers::Container {
                width: 4,
                height: 4,
                capacity: 16,
                max_item: 4,
                children: Vec::new(),
            }),
            random_table: None,
            has_body: false,
            has_weapon: true,
            has_shield: false,
            coverage_units: [0; 20],
            weapon_word: 0,
            shield_word: 0,
        }
    }

    /// The records come from the live database: a kit's byte written
    /// there reaches the decoded record of that id only, and a tagged
    /// copy's container list is its own.
    #[test]
    fn the_records_follow_the_live_database() {
        use exa_formats::objdb::{ObjectDb, Store};
        let section = |tag: u32, body: &[u8]| {
            let mut out = tag.to_le_bytes().to_vec();
            out.extend((body.len() as u32 + 8).to_le_bytes());
            out.extend_from_slice(body);
            out
        };
        let mut weapon = vec![0u8; 0x5A];
        weapon[0x0D] = 0x60;
        let mut container = vec![0u8; 0x18];
        container[0] = 4;
        let mut payload = section(0x1000_0000, &[0u8; 24]);
        payload.extend(section(0x4000, &weapon));
        payload.extend(section(exa_formats::objdb::CONTAINER, &container));
        let mut db = ObjectDb::default();
        db.base = Store::from_records([(0x12, 0x4000, payload.as_slice())]);
        db.local = Store::from_records([(0x12, 0x4000, payload.as_slice())]);
        let mut live = LiveObjects::of(db);
        let mut items = DollItems::default();
        std::sync::Arc::make_mut(&mut items.items).insert(0x12, weapon_item(0x12));
        items.sync(&live);
        let copy = 0x8000_0012;
        assert_eq!(items.get(copy).unwrap().weapon.unwrap().damage, 0x60);
        live.write_condition(copy, false, false, 0x10);
        items.sync(&live);
        assert_eq!(items.get(copy).unwrap().weapon.unwrap().damage, 0x10);
        assert_eq!(items.get(0x12).unwrap().weapon.unwrap().damage, 0x60);
        let record = crate::item_container::ContainerRecord {
            children: vec![crate::item_container::ContainerChild {
                id: 0x99,
                ..Default::default()
            }],
            ..Default::default()
        };
        items.store_container(&mut live, copy, &record);
        let children = |items: &DollItems, id| {
            items
                .get(id)
                .and_then(|item| item.container.as_ref())
                .map_or(0, |c| c.children.len())
        };
        assert_eq!(children(&items, copy), 1);
        assert_eq!(children(&items, 0x12), 0);
        // A level load swaps the tagged records out.
        live.load_level("next", None, None);
        items.sync(&live);
        assert!(items.copies.is_empty());
    }

    /// A torch or off-hand root on the figure takes the pick's marker with
    /// its 1-based slot; a striker or the figure's shield does not.
    /// The view's scene, made by the live sync when the player's inventory
    /// opens, renders at the settings' supersampling: two image pixels a
    /// view pixel with super sampling on, one with it off.
    #[test]
    fn the_view_image_follows_the_supersampling_setting() {
        for (on, scale) in [(true, 2), (false, 1)] {
            let mut app = App::new();
            app.init_resource::<GameScreens>()
                .init_resource::<InventoryDoll>()
                .init_resource::<Assets<Image>>()
                .init_resource::<crate::char_view::PreviewPointer>()
                .insert_resource(exa_render::supersample::SuperSampling(on))
                .add_systems(Update, sync_doll);
            let view = {
                let mut screens = app.world_mut().resource_mut::<GameScreens>();
                screens.open_inventory(true, true, 4);
                open_view(&screens).unwrap()
            };
            app.update();
            let doll = app.world().resource::<InventoryDoll>();
            let scene = doll.scene.as_ref().expect("the view's scene");
            assert_eq!(scene.scale, scale);
            let image = app
                .world()
                .resource::<Assets<Image>>()
                .get(&scene.image)
                .unwrap();
            assert_eq!(
                image.size(),
                image_size_at((view.width(), view.height()), scale)
            );
        }
    }

    #[test]
    fn figure_hand_roots_take_the_pick_marker() {
        use crate::figure_pick::FigureHeld;
        assert_eq!(figure_held_mark(0, false, false), Some(FigureHeld(1)));
        assert_eq!(figure_held_mark(1, false, false), Some(FigureHeld(2)));
        assert_eq!(figure_held_mark(0, true, false), None);
        assert_eq!(figure_held_mark(1, false, true), None);
    }

    /// The view is the doll panel less its 2-pixel border: 224 × 380 at
    /// unit 4 for the constructor's 228 × 384 panel.
    #[test]
    fn the_view_is_the_panel_less_its_border() {
        let view = view_rect(GuiRect::new(16, 16, 228, 384), 2);
        assert_eq!((view.left, view.top), (18, 18));
        assert_eq!((view.width(), view.height()), (224, 380));
        assert_eq!(image_size((224, 380)), UVec2::new(448, 760));
        // The inventory's view takes the settings' level: 4x is a 2 x 2
        // grid, off is one sample.
        assert_eq!(doll_scale(true), 2);
        assert_eq!(doll_scale(false), 1);
        assert_eq!(
            image_size_at((224, 380), doll_scale(false)),
            UVec2::new(224, 380)
        );
    }

    /// The dots centre on the record: a 28-pixel record puts the 22-pixel
    /// dots 3 down; a lower record's negative difference shifts as an
    /// unsigned value and truncates (−2 to −1, −4 to −2).
    #[test]
    fn the_dots_centre_on_the_record() {
        assert_eq!(dot_top(386, 28, 22), 389);
        assert_eq!(dot_top(386, 22, 22), 386);
        assert_eq!(dot_top(386, 20, 22), 385);
        assert_eq!(dot_top(386, 18, 22), 384);
    }

    /// The camera stands 4.10 m from the figure's point (1 m above its
    /// feet), toward (−200, 60, 360), looking at the point; the light comes
    /// from the camera-space (0.5, 0.7, 0.5).
    #[test]
    fn the_camera_frames_the_figure_from_the_views_eye() {
        let camera = camera_transform();
        let point = DOLL_ORIGIN + Vec3::Y;
        let distance = camera.translation.distance(point);
        assert!((distance - 4.1).abs() < 1e-3, "{distance}");
        let toward = (camera.translation - point).normalize();
        let eye = Vec3::new(-200.0, 60.0, 360.0).normalize();
        assert!(toward.abs_diff_eq(eye, 1e-4), "{toward:?}");
        assert!(camera.forward().as_vec3().abs_diff_eq(-eye, 1e-4));
        let light = light_transform(&camera);
        let from = camera.rotation * Vec3::new(0.5, 0.7, 0.5).normalize();
        assert!(light.forward().as_vec3().abs_diff_eq(-from, 1e-4));
    }

    /// The take-in plan: a sword into empty hands goes to the main hand; a
    /// second sword replaces it (its carried copy losing the held mark); a
    /// two-hander frees both hands; a shield frees the off hand.
    #[test]
    fn items_go_into_the_hands_by_the_take_in_plan() {
        let sword = possession::ItemKind {
            weapon: Some(0x10),
            ..Default::default()
        };
        let great = possession::ItemKind {
            weapon: Some(0x20),
            ..Default::default()
        };
        let kinds = |id: u32| match id {
            5 | 6 => Some(sword),
            7 => Some(possession::ItemKind {
                shield: true,
                ..Default::default()
            }),
            _ => None,
        };
        let mut p = Possessions::default();
        p.carried.push(possession::Carried {
            id: 5,
            flags: HELD,
            x: 0,
            y: 0,
        });
        hold_item(&mut p, 5, 0x10, sword, false, kinds);
        // The attach job keeps only the top bit of the item's flags.
        assert_eq!(p.out[0], Hand { id: 5, flags: 0 });
        hold_item(&mut p, 6, 0x10, sword, false, kinds);
        assert_eq!(p.out[0], Hand { id: 6, flags: 0 });
        assert_eq!(p.carried[0].flags, 0);
        let taken = hold_shield(&mut p, 7, 0, kinds);
        assert_eq!(p.out[1], Hand { id: 7, flags: 0 });
        // A one-hander in the main hand: its class bits keep the part.
        assert!(!taken.clears_part && !taken.frees_off);
        hold_item(&mut p, 8, 0x20, great, false, kinds);
        assert_eq!(p.out, [Hand { id: 8, flags: 0 }, Hand::default()]);
        // A one-hander asked for the off hand (the pointer's right half
        // with Insight's technique 8) goes there beside the main hand.
        hold_item(&mut p, 5, 0, sword, true, kinds);
        assert_eq!(p.out, [Hand { id: 8, flags: 0 }, Hand { id: 5, flags: 0 }]);
        // Without the request a one-hander goes into the main hand.
        hold_item(&mut p, 9, 0x10, sword, false, kinds);
        assert_eq!(p.out[0], Hand { id: 9, flags: 0 });
        assert_eq!(class_word(Some(0x10)), 0x10);
        assert_eq!(class_word(Some(0x30)), 0x20);
        assert_eq!(class_word(None), 0x80);
    }

    /// A lit torch dropped onto the doll: the record keeps only the top
    /// bit, the held part's mark keeps the light, and the first toggle puts
    /// it out (the bit stays clear and the mark turns over).
    #[test]
    fn a_lit_torch_dropped_on_the_doll_goes_out_on_the_first_toggle() {
        let torch = possession::ItemKind {
            weapon: Some(0x13),
            light: true,
            ..Default::default()
        };
        let mut p = Possessions::default();
        hold_item(
            &mut p,
            0x1f,
            possession::LIT | 0x8000_0000,
            torch,
            false,
            |_| Some(torch),
        );
        assert_eq!(
            p.out[0],
            Hand {
                id: 0x1f,
                flags: 0x8000_0000
            }
        );
        let mark = held_mark(0x1f, possession::LIT | 0x8000_0000);
        assert!(mark.lit);
        let burning = p.burning(Some(&mark));
        assert!(burning);
        assert!(p.toggle_light(true, false, true, burning));
        assert_eq!(p.out[0].flags & possession::LIT, 0);
        // The next toggle reads the turned mark and lights it again.
        let mark = possession::HeldMark {
            item: p.first_held(),
            lit: !burning,
        };
        assert!(!p.burning(Some(&mark)));
        assert!(p.toggle_light(true, false, true, p.burning(Some(&mark))));
        assert_ne!(p.out[0].flags & possession::LIT, 0);
    }

    #[test]
    fn the_doll_drop_script_reads_item_flags_and_frame() {
        let entry = |id, flags, frame, near| DollDropEntry {
            id,
            flags,
            frame,
            near,
        };
        assert_eq!(
            DollDropScript::parse("0x1f:0x2000000@120"),
            Some(DollDropScript(vec![entry(0x1f, 0x0200_0000, 120, false)]))
        );
        assert_eq!(
            DollDropScript::parse("0x1f@3,0x20@9+"),
            Some(DollDropScript(vec![
                entry(0x1f, 0, 3, false),
                entry(0x20, 0, 9, true)
            ]))
        );
        assert_eq!(DollDropScript::parse("0x1f"), None);
        assert_eq!(DollDropScript::parse(""), None);
    }

    /// The figure's targets: the record's hands, with the carried torch in
    /// its own hand when that hand is free, else in the other free one.
    #[test]
    fn the_carried_torch_fills_a_free_hand_of_the_figure() {
        let sword = Hand { id: 0x20, flags: 0 };
        let shield = Hand { id: 0x30, flags: 0 };
        let torch = Hand {
            id: crate::flow::STORY_TORCH_ITEM,
            flags: possession::LIT,
        };
        let none = Hand::default();
        assert_eq!(figure_targets(&[none, none], Some(0)), [torch, none]);
        assert_eq!(figure_targets(&[sword, none], Some(1)), [sword, torch]);
        // The torch's own hand taken: the other one.
        assert_eq!(figure_targets(&[sword, none], Some(0)), [sword, torch]);
        // Both taken, or no torch: the record alone.
        assert_eq!(figure_targets(&[sword, shield], Some(1)), [sword, shield]);
        assert_eq!(figure_targets(&[sword, none], None), [sword, none]);
        let lit = |main, off_light| Hands {
            main,
            off_light,
            ..Hands::default()
        };
        let torch_item = exa_character::held::HeldItem {
            flags: crate::torch::TORCH_FLAGS,
            light: true,
        };
        assert_eq!(torch_hand(&lit(Some(torch_item), false), true), Some(0));
        assert_eq!(torch_hand(&lit(None, true), true), Some(1));
        assert_eq!(torch_hand(&lit(Some(torch_item), false), false), None);
        assert_eq!(torch_hand(&lit(None, false), true), None);
    }

    /// A shield hangs on the left hand turned -5° about z; a second weapon
    /// takes the off-hand grip and the main weapon the first slot's; a
    /// light takes the torch's grip in its hand.
    #[test]
    fn hand_models_hang_by_their_attach_kind() {
        let holder = exa_combat::grip::GripHolder::HUMAN;
        let (bone, shield) = hand_placement(AttachKind::Shield, 0, None, false, &holder);
        assert_eq!(bone, exa_character::LEFT_HAND);
        assert!(
            shield
                .rotation
                .angle_between(Quat::from_rotation_z((-5.0f32).to_radians()))
                < 1e-5
        );
        let sword = Some(0x11);
        let (main, _) = hand_placement(AttachKind::Weapon, 0, sword, false, &holder);
        let (off, _) = hand_placement(AttachKind::Weapon, 1, sword, false, &holder);
        assert_eq!(main, exa_character::RIGHT_HAND);
        assert_eq!(off, exa_character::LEFT_HAND);
        let (torch_main, at_main) = hand_placement(AttachKind::Light, 0, None, false, &holder);
        let (torch_off, at_off) = hand_placement(AttachKind::Light, 1, None, false, &holder);
        assert_eq!(torch_main, crate::torch::TorchHand::Main.bone());
        assert_eq!(torch_off, crate::torch::TorchHand::Off.bone());
        assert_eq!(
            at_main,
            crate::torch::TorchHand::Main
                .grip()
                .placement(&holder, false)
        );
        assert_eq!(
            at_off,
            crate::torch::TorchHand::Off
                .grip()
                .placement(&holder, false)
        );
    }

    /// The dots draw five whole dots and a half at 5.5, 12 pixels apart at
    /// unit 4, three units down from a 28-pixel record's top.
    #[test]
    fn the_rating_dots_draw_whole_and_half_dots() {
        let cell = vec![255u8; 64 * 64 * 4];
        let sheet = DotSheet::of_cells(&[cell.clone(), cell], 64, 22).unwrap();
        let layer = rating_dots_layer(&sheet, 5.5, (100, 386, 28), 4, [1, 2, 3, 4]).unwrap();
        // Six quads of four vertices each.
        assert_eq!(layer.batch.vertices.len(), 24);
        let xs: Vec<f32> = layer
            .batch
            .vertices
            .iter()
            .step_by(4)
            .map(|v| v.x)
            .collect();
        assert_eq!(xs, vec![100.0, 112.0, 124.0, 136.0, 148.0, 160.0]);
        assert!(
            layer
                .batch
                .vertices
                .iter()
                .all(|v| v.y == 389.0 || v.y == 411.0)
        );
        assert!(rating_dots_layer(&sheet, 0.0, (0, 0, 28), 4, [0; 4]).is_none());
    }

    #[test]
    fn the_figure_takes_race_bound_items_only_for_their_race() {
        let mut human = exa_formats::races::builtin_human();
        human.look = 0x1BF;
        let mut golem = human.clone();
        golem.look = 0;
        golem.flags = 0x2000;
        golem.stats.kind = 0x1_0000;
        let golem_weapon = possession::ItemKind {
            weapon: Some(0x0200_0000),
            ..Default::default()
        };
        let large_weapon = possession::ItemKind {
            weapon: Some(0x0400_0000),
            ..Default::default()
        };
        let shield = possession::ItemKind {
            shield: true,
            ..Default::default()
        };
        let plain = possession::ItemKind::default();
        let takes = |race, id, kind, shield, apparel, frozen| {
            figure_accepts(Some(race), id, kind, shield, apparel, frozen)
        };
        // A weapon flagged for golems: only a race with the kind bit 0x10000.
        assert!(!takes(&human, 7, golem_weapon, None, false, false));
        assert!(takes(&golem, 7, golem_weapon, None, false, false));
        // A weapon flagged for the large races needs the bit 0x20000.
        assert!(!takes(&golem, 7, large_weapon, None, false, false));
        // A shield reads its own section's flags.
        assert!(takes(&human, 9, shield, Some(0), false, false));
        assert!(!takes(&human, 9, shield, Some(0x0200_0000), false, false));
        // Apparel: the look word's bit 4 and a view that allows changes.
        assert!(takes(&human, 8, plain, None, true, false));
        assert!(!takes(&human, 8, plain, None, true, true));
        assert!(!takes(&golem, 8, plain, None, true, false));
        // No section: only the AL crystal on a race that takes it.
        assert!(!takes(&human, 500, plain, None, false, false));
        assert!(takes(&golem, 500, plain, None, false, false));
        assert!(!takes(&golem, 501, plain, None, false, false));
        // No race table: the human's look.
        assert!(figure_accepts(None, 8, plain, None, true, false));
    }

    /// The shipped records' race-bound weapons and shields, tested against
    /// the shipped races (skipped without the game).
    #[test]
    fn shipped_race_bound_items() {
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let pack = game.pack(exa_rpk::PackKind::Resource).unwrap();
        let db = exa_formats::rdb::Database::parse(pack.get("objdb.rdb").unwrap()).unwrap();
        let catalogue = ItemCatalogue::of_database(&db);
        let races = exa_formats::races::RaceDb::new(
            exa_formats::races::parse(pack.get("races.rdb").unwrap()).unwrap(),
        );
        let human = races.of_character(0);
        let mut bound = Vec::new();
        for r in &db.records {
            let kind = catalogue.kind(r.id).unwrap_or_default();
            // A weapon is bound by either bit, a shield by the golems' only
            // (one shield record's first word is text whose 0x4000000 bit
            // the test does not read for shields).
            let (flags, mask) = match (kind.weapon, catalogue.shield_flags(r.id)) {
                (Some(flags), _) => (flags, 0x0600_0000),
                (None, Some(flags)) if kind.shield => (flags, 0x0200_0000),
                _ => continue,
            };
            if flags & mask == 0 {
                continue;
            }
            let takers: Vec<&str> = races
                .races
                .iter()
                .filter(|race| {
                    figure_accepts(
                        Some(race),
                        r.id,
                        kind,
                        catalogue.shield_flags(r.id),
                        false,
                        false,
                    )
                })
                .map(|race| race.name.as_str())
                .collect();
            eprintln!("{:#x} flags {flags:#x}: taken by {takers:?}", r.id);
            assert!(!figure_accepts(
                Some(human),
                r.id,
                kind,
                catalogue.shield_flags(r.id),
                false,
                false
            ));
            bound.push(r.id);
        }
        eprintln!("{} race-bound weapons and shields", bound.len());
        assert!(!bound.is_empty());
    }

    /// A shield dragged from the player's pack onto the figure, while
    /// another is in the off hand: the dragged entry stays in the pack at
    /// its cell, marked held, and the displaced shield's held entry shows
    /// again (shipped records, skipped without the game).
    #[test]
    fn a_shield_from_the_pack_is_held_in_place_and_the_old_one_comes_back() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let Ok(data) = exa_assets::GameData::open(&dir) else {
            return;
        };
        let Some(bytes) = data
            .pack(PackKind::Resource)
            .and_then(|pack| pack.get("objdb.rdb"))
        else {
            return;
        };
        let Ok(db) = exa_formats::rdb::Database::parse(bytes) else {
            return;
        };
        let catalogue = ItemCatalogue::of_database(&db);
        let mut shields = exa_formats::objdb::items(&db)
            .into_iter()
            .map(|item| item.id)
            .filter(|&id| {
                catalogue
                    .kind(id)
                    .is_some_and(|k| k.shield && k.weapon.is_none())
            });
        let (Some(old), Some(new)) = (shields.next(), shields.next()) else {
            return;
        };
        let mut app = App::new();
        app.init_resource::<GameScreens>()
            .init_resource::<PanelIcons>()
            .init_resource::<DollItems>()
            .insert_resource(catalogue)
            .insert_resource(Game(std::sync::Arc::new(data)))
            .add_message::<DollDrop>()
            .init_resource::<crate::thaum::live::FrameEffectRoll>()
            .add_systems(Update, equip_dropped);
        let player = app
            .world_mut()
            .spawn((
                crate::player::PlayerBody::new(0.0),
                Possessions {
                    out: [Hand::default(), Hand { id: old, flags: 0 }],
                    carried: vec![
                        possession::Carried {
                            id: old,
                            flags: HELD,
                            x: 0,
                            y: 0,
                        },
                        possession::Carried {
                            id: new,
                            flags: 0,
                            x: 320,
                            y: 160,
                        },
                    ],
                    ..Default::default()
                },
            ))
            .id();
        let world = app.world_mut();
        let possessions = world.get::<Possessions>(player).unwrap().clone();
        world.resource_scope(|world, mut icons: Mut<PanelIcons>| {
            let catalogue = world.resource::<ItemCatalogue>().clone();
            let mut screens = world.resource_mut::<GameScreens>();
            screens.task.screen = (1280, 720);
            screens.open_inventory(true, true, 4);
            screens.inventory_body = Some(player);
            body_inventory::bind_pack(
                &mut screens.pack,
                &possessions,
                &mut icons,
                Some(&catalogue),
            );
        });
        let (cell, icon) = {
            let mut screens = app.world_mut().resource_mut::<GameScreens>();
            let entry = screens
                .pack
                .entries
                .iter_mut()
                .find(|e| e.id == new)
                .unwrap();
            entry.drag = 1;
            (entry.cell, entry.icon)
        };
        assert!(
            app.world()
                .resource::<GameScreens>()
                .pack
                .entries
                .iter()
                .any(|e| e.id == old && e.flags & HELD != 0)
        );
        app.world_mut().write_message(DollDrop {
            source: Panel::PlayerPack,
            id: new,
            flags: 0,
            icon,
            bulk: 0,
            right_half: false,
        });
        app.update();
        let p = app.world().get::<Possessions>(player).unwrap();
        assert_eq!(p.out[1].id, new);
        let screens = app.world().resource::<GameScreens>();
        let held = screens.pack.entries.iter().find(|e| e.id == new).unwrap();
        assert_eq!((held.cell, held.drag, held.flags & HELD), (cell, 0, HELD));
        let back = screens.pack.entries.iter().find(|e| e.id == old).unwrap();
        assert_eq!(back.flags & HELD, 0);
        assert_eq!(screens.pack.entries.len(), 2);
        assert!(p.carried.iter().any(|c| c.id == old && c.flags & HELD == 0));
        assert!(p.carried.iter().any(|c| c.id == new && c.flags & HELD != 0));
    }

    /// The shown item's icon sits at the view's lower-left corner: the
    /// draw origin of (unit, height less unit) and the hover's test point
    /// agree, so the pointer is on it exactly where it draws.
    #[test]
    fn the_shown_icon_draws_and_answers_at_the_views_lower_left() {
        let mut icons = PanelIcons::new(4);
        let mut image = IconImage::new(160, 160, 4);
        image.pixels.fill(255);
        let cell = icons.atlas.add(&image, 7);
        icons.cache.entries = vec![exa_items::icon_cache::Entry {
            id: 500,
            cell,
            variant: 0,
            half_width: 84,
            half_height: 84,
            refs: 1,
        }];
        // A 380-pixel view at unit 4 with 160-pixel icons.
        assert_eq!(shown_icon_origin(&icons, 1, 4, 380, 160), Some((8, 212)));
        // The icon space's rows run up from its bottom edge, so its top
        // row and its left column start one pixel in, as in a panel.
        assert!(over_shown_icon(&icons, 1, (9, 213), 4, 380, 160));
        assert!(over_shown_icon(&icons, 1, (167, 371), 4, 380, 160));
        assert!(!over_shown_icon(&icons, 1, (7, 300), 4, 380, 160));
        assert!(!over_shown_icon(&icons, 1, (100, 211), 4, 380, 160));
        assert!(!over_shown_icon(&icons, 0, (100, 300), 4, 380, 160));
    }

    /// The inventory's view on a body whose race takes the AL crystal as
    /// its core (the shipped race table): the bind shows the first worn
    /// crystal, none for a body that wears none; the AL crystal dropped
    /// from the pack leaves it and is shown over the figure, and a second
    /// one is refused while one is shown.
    #[test]
    fn the_view_shows_the_core_crystal_and_takes_one_dropped_on_it() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let Ok(data) = exa_assets::GameData::open(&dir) else {
            return;
        };
        let Some(bytes) = data
            .pack(PackKind::Resource)
            .and_then(|pack| pack.get("objdb.rdb"))
        else {
            return;
        };
        let Ok(db) = exa_formats::rdb::Database::parse(bytes) else {
            return;
        };
        let catalogue = ItemCatalogue::of_database(&db);
        let game = Game(std::sync::Arc::new(data));
        let races = exa_formats::races::RaceDb::new(crate::scene_bodies::game_races(&game));
        let Some(golem) =
            (0..=u8::MAX).find(|&b| races.of_character(b).flags & possession::TAKES_CORE_AL != 0)
        else {
            panic!("the shipped race table has a race that takes the AL crystal");
        };
        let al = possession::crystal::AL;
        // The crystal is an item with no apparel, weapon or shield section.
        let kind = catalogue.kind(al).unwrap_or_default();
        assert!(kind.weapon.is_none() && !kind.shield);
        let mut app = App::new();
        app.init_resource::<GameScreens>()
            .init_resource::<PanelIcons>()
            .init_resource::<DollItems>()
            .init_resource::<InventoryDoll>()
            .insert_resource(catalogue)
            .insert_resource(game)
            .add_message::<DollDrop>()
            .init_resource::<crate::thaum::live::FrameEffectRoll>()
            .add_systems(
                Update,
                (
                    bind_shown_item,
                    equip_dropped,
                    crate::equip::refresh_worn_specials,
                )
                    .chain(),
            );
        let carried = |x| possession::Carried {
            id: al,
            flags: 0x0a_0000,
            x,
            y: 0,
        };
        let body = app
            .world_mut()
            .spawn((
                crate::player::PlayerBody::new(0.0),
                Possessions {
                    carried: vec![carried(0), carried(320)],
                    ..Default::default()
                },
                exa_character::record::CharacterData::of_record(
                    exa_formats::characters::CharacterRecord {
                        race: golem,
                        ..Default::default()
                    },
                ),
                {
                    // A golem without its core: the armour pass's mark.
                    let mut fighter = exa_combat::Fighter::default();
                    fighter.stats.golem_core = Some(al);
                    fighter.stats.rules.0 = crate::equip::CORELESS | 0x40;
                    fighter
                },
            ))
            .id();
        let rules = |app: &App| {
            app.world()
                .get::<exa_combat::Fighter>(body)
                .unwrap()
                .stats
                .rules
                .0
        };
        let scene = |body| DollScene {
            image: Handle::default(),
            camera: Entity::PLACEHOLDER,
            light: Entity::PLACEHOLDER,
            node: Entity::PLACEHOLDER,
            figure: None,
            body: Some(body),
            view: GuiRect::new(0, 0, 224, 380),
            backdrop: [0; 4],
            scale: 2,
        };
        let world = app.world_mut();
        let possessions = world.get::<Possessions>(body).unwrap().clone();
        world.resource_scope(|world, mut icons: Mut<PanelIcons>| {
            let catalogue = world.resource::<ItemCatalogue>().clone();
            let mut screens = world.resource_mut::<GameScreens>();
            screens.task.screen = (1280, 720);
            screens.open_inventory(true, true, 4);
            screens.inventory_body = Some(body);
            body_inventory::bind_pack(
                &mut screens.pack,
                &possessions,
                &mut icons,
                Some(&catalogue),
            );
        });
        app.world_mut().resource_mut::<InventoryDoll>().scene = Some(scene(body));
        app.update();
        assert_eq!(app.world().resource::<InventoryDoll>().shown, None);
        let drop = |app: &mut App| {
            let (icon, flags) = {
                let mut screens = app.world_mut().resource_mut::<GameScreens>();
                let entry = screens
                    .pack
                    .entries
                    .iter_mut()
                    .find(|e| e.id == al && e.drag == 0)
                    .unwrap();
                entry.drag = 1;
                (entry.icon, entry.flags)
            };
            app.world_mut().write_message(DollDrop {
                source: Panel::PlayerPack,
                id: al,
                flags,
                icon,
                bulk: 0,
                right_half: false,
            });
            app.update();
        };
        drop(&mut app);
        let shown = app.world().resource::<InventoryDoll>().shown.unwrap();
        assert_eq!((shown.id, shown.flags), (al, 0x0a_0000));
        assert!(shown.icon > 0);
        let p = app.world().get::<Possessions>(body).unwrap();
        assert_eq!(p.carried.iter().filter(|c| c.id == al).count(), 1);
        assert!(p.worn.is_empty(), "the record's lists do not take the core");
        // The drop clears the body's coreless mark, and the pack's change
        // runs no armour pass that would set it again.
        assert_eq!(rules(&app), 0x40);
        app.update();
        assert_eq!(rules(&app), 0x40);
        // A second crystal is refused while one is shown.
        drop(&mut app);
        let p = app.world().get::<Possessions>(body).unwrap();
        assert_eq!(p.carried.iter().filter(|c| c.id == al).count(), 1);
        // The view closes and binds again on a body wearing a crystal.
        app.world_mut().resource_mut::<InventoryDoll>().scene = None;
        app.update();
        assert_eq!(app.world().resource::<InventoryDoll>().shown, None);
        app.world_mut()
            .get_mut::<Possessions>(body)
            .unwrap()
            .worn
            .push(ItemRef::new(possession::crystal::A, 0x05_0000));
        app.world_mut().resource_mut::<InventoryDoll>().scene = Some(scene(body));
        app.update();
        let shown = app.world().resource::<InventoryDoll>().shown.unwrap();
        assert_eq!((shown.id, shown.flags), (possession::crystal::A, 0x05_0000));
    }

    /// A weapon dropped on a bound body's figure, with the inventory's
    /// mind window set, raises the body's trust by twice its desirability
    /// less what it displaces (an empty hand: 0). Against a hostile body
    /// (its limit 10) the 4.8 is not enough: the binding ends, the window
    /// closes and the weapon is not taken.
    #[test]
    fn a_weapon_drop_on_a_bound_body_changes_its_trust() {
        use exa_formats::objdb::{ObjectDb, Store};
        let run = |relation: f32| {
            let payload = crate::view_trust::weapon_payload();
            let mut db = ObjectDb::default();
            db.local = Store::from_records([(0x21, 0x4000, payload.as_slice())]);
            let weapon = exa_formats::objdb::LOCAL | 0x21;
            let mut catalogue = ItemCatalogue::default();
            catalogue.set_level_cards(&[(0x21, payload.clone())], None);
            let mut app = App::new();
            app.init_resource::<GameScreens>()
                .init_resource::<PanelIcons>()
                .init_resource::<DollItems>()
                .insert_resource(catalogue)
                .insert_resource(LiveObjects::of(db))
                .add_message::<DollDrop>()
                .init_resource::<crate::thaum::live::FrameEffectRoll>()
                .add_systems(Update, equip_dropped);
            let (_, body) = crate::view_trust::bound_pair(app.world_mut(), relation);
            {
                let mut screens = app.world_mut().resource_mut::<GameScreens>();
                screens.task.screen = (1280, 720);
                screens.show_inventory(true, true, 4, true, 3);
                screens.inventory_body = Some(body);
                screens.inventory_mind_window = true;
            }
            app.world_mut().write_message(DollDrop {
                source: Panel::Container(0x99),
                id: weapon,
                flags: 0,
                icon: 0,
                bulk: 0,
                right_half: false,
            });
            app.update();
            let held = app.world().get::<Possessions>(body).unwrap().out[0].id == weapon;
            let open = app
                .world()
                .resource::<GameScreens>()
                .find(exa_ui::game_gui::GameScreen::Inventory)
                .is_some();
            let (trust, runs) = crate::view_trust::marker_trust(app.world(), body);
            (held, open, trust, runs)
        };
        let (held, open, trust, runs) = run(0.0);
        assert!(held && open && runs);
        assert!((trust - 4.8).abs() < 1e-5);
        let (held, open, trust, runs) = run(-100.0);
        assert!(!held && !open && !runs);
        assert!((trust - 4.8).abs() < 1e-5);
    }

    /// The displaced weight follows the take-in's branches: a one-handed
    /// weapon weighs the main hand's item; a two-handed one both hands'.
    #[test]
    fn the_displaced_weight_reads_the_record_hands() {
        let one = possession::ItemKind {
            weapon: Some(0x10),
            ..Default::default()
        };
        let two = possession::ItemKind {
            weapon: Some(0x20),
            ..Default::default()
        };
        let p = Possessions {
            out: [Hand { id: 1, flags: 0 }, Hand { id: 2, flags: 0 }],
            ..Default::default()
        };
        let value = |id: u32| id as f32 * 10.0;
        let kind_of = |_| Some(one);
        assert_eq!(displaced_by(&p, one, false, None, value, kind_of), 10.0);
        assert_eq!(displaced_by(&p, two, false, None, value, kind_of), 30.0);
        assert_eq!(
            displaced_by(&Possessions::default(), one, false, None, value, kind_of),
            0.0
        );
        // A light taken in reads the body's own class word: a one-handed
        // class weighs the off hand, a two-handed body's the main hand,
        // whatever the main hand's grip would suggest.
        let torch = possession::ItemKind { light: true, ..one };
        assert_eq!(displaced_by(&p, torch, false, None, value, kind_of), 20.0);
        assert_eq!(
            displaced_by(&p, torch, false, Some(0x10), value, kind_of),
            20.0
        );
        assert_eq!(
            displaced_by(&p, torch, false, Some(0x20), value, kind_of),
            10.0
        );
    }

    #[test]
    fn the_dots_take_the_gui_levels_large_icon_cell() {
        assert_eq!(dot_side(3), 16);
        assert_eq!(dot_side(4), 20);
        assert_eq!(dot_side(5), 28);
        assert_eq!(dot_side(12), 64);
        let cell = vec![255u8; 64 * 64 * 4];
        let sheet = DotSheet::of_cells(&[cell.clone(), cell], 64, dot_side(4)).unwrap();
        assert_eq!((sheet.side, sheet.atlas.cell), (20, 20));
        // A 28-pixel row centres 20-pixel dots 4 below its top.
        let layer = rating_dots_layer(&sheet, 1.0, (100, 386, 28), 4, [1, 2, 3, 4]);
        assert!(layer.is_some());
        assert_eq!(dot_top(386, 28, sheet.side), 390);
        let knob = exa_items::item_gui::slider(0, 200, 0.5, sheet.side);
        assert_eq!(knob.knob, 100);
        assert_eq!(knob.left, Some((10, 106)));
    }
}
