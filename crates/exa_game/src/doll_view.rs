//! The inventory's character view: the player's dressed figure drawn by a
//! 3D view of its own inside the window's doll panel, the encumbrance dots
//! beside the Encumbrance label, and the drop that dresses the player from
//! a dragged item.
//!
//! The figure is a copy of the player's body (its model, physique, skin,
//! hair, outfit and main-hand weapon) standing at its own place far from
//! the world, on a render layer no other camera takes. An off-screen
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
//! inventory's own client showing through; the figure keeps the player's
//! outfit and weapon but not its torch.

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
use exa_assets::model::{self, ModelLoader};
use exa_assets::{Game, ModelCache, PackKind};
use exa_character::held::Hands;
use exa_character::{Garment, Outfit, Wielded};
use exa_formats::characters::{ID_MASK, ItemRef};
use exa_formats::objdb::Item;
use exa_items::catalogue::ItemCatalogue;
use exa_items::doll::{IntoFacts, record_into, take_into_hands};
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
pub const DOLL_SCALE: u32 = 2;
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
    UVec2::new(
        (size.0.max(1) as u32) * DOLL_SCALE,
        (size.1.max(1) as u32) * DOLL_SCALE,
    )
}

/// The dot sheet's whole and half dots resampled to the interface's large
/// icon cell ([`exa_items::item_gui::DOT`]) in an atlas of their own.
pub struct DotSheet {
    pub atlas: IconAtlas,
    pub texture: GuiTexture,
    pub whole: i32,
    pub half: i32,
}

impl DotSheet {
    /// From the sheet's decoded cells (bottom row first, 64 pixels square,
    /// whole dot first): `None` unless both draw.
    pub fn of_cells(cells: &[Vec<u8>], cell: usize) -> Option<Self> {
        let side = exa_items::item_gui::DOT as usize;
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
    let top = dot_top(y, height, exa_items::item_gui::DOT) as f32;
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
    /// The view's rectangle the scene was made for.
    pub view: GuiRect,
    /// The weapon model the figure holds, to follow the player's.
    pub weapon: Option<String>,
    /// The colour the camera clears to.
    pub backdrop: [u8; 4],
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
    sheet_tried: bool,
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

/// The game's object records by id, read once when a drop first needs an
/// apparel's occupancy and worn model.
#[derive(Resource, Default)]
pub struct DollItems {
    items: HashMap<u32, Item>,
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
        self.items = items.into_iter().map(|item| (item.id, item)).collect();
    }

    pub fn get(&self, id: u32) -> Option<&Item> {
        self.items.get(&(id & ID_MASK))
    }
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
) -> DollScene {
    let size = image_size((view.width(), view.height()));
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
        view,
        weapon: None,
        backdrop,
    }
}

/// The view follows the inventory: made when it opens (and again when the
/// view's size changes), placed over the doll panel, its camera's clear
/// kept at the backdrop, the pointer handed to the figure's turn, and
/// everything dropped when it closes.
#[allow(clippy::too_many_arguments)]
fn sync_doll(
    mut commands: Commands,
    screens: Res<GameScreens>,
    mut doll: ResMut<InventoryDoll>,
    mut images: ResMut<Assets<Image>>,
    mut pointer: ResMut<crate::char_view::PreviewPointer>,
    windows: Query<&Window, With<PrimaryWindow>>,
    players: Query<&exa_combat::Fighter, With<crate::player::PlayerBody>>,
    mut nodes: Query<&mut Node>,
    mut cameras: Query<&mut Camera>,
    game: Option<Res<Game>>,
) {
    let Some(view) = open_view(&screens) else {
        if let Some(scene) = doll.scene.take() {
            despawn_scene(&mut commands, scene);
            pointer.0 = None;
        }
        doll.dots = None;
        return;
    };
    doll.dots = players
        .iter()
        .next()
        .map(|fighter| exa_items::inventory_window::encumbrance_dots(fighter.stats.encumbrance));
    if !doll.sheet_tried
        && let Some(game) = game.as_deref()
    {
        doll.sheet_tried = true;
        doll.sheet = load_dot_sheet(game);
    }
    let resized = doll.scene.as_ref().is_some_and(|scene| {
        (scene.view.width(), scene.view.height()) != (view.width(), view.height())
    });
    if resized && let Some(scene) = doll.scene.take() {
        despawn_scene(&mut commands, scene);
    }
    let backdrop = doll.backdrop;
    let scene = doll
        .scene
        .get_or_insert_with(|| spawn_scene(&mut commands, &mut images, view, backdrop));
    scene.view = view;
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

/// Reads the dot sheet and keeps its whole and half dots.
fn load_dot_sheet(game: &Game) -> Option<DotSheet> {
    let sheet = &exa_items::item_gui::SPRITE_SHEETS[0];
    let name = sheet.name.strip_suffix(".rfi").unwrap_or(sheet.name);
    let bytes = game.0.texture(name)?;
    let loaded = exa_items::item_gui::load_sheet(sheet, bytes)?;
    DotSheet::of_cells(&loaded.cells, sheet.cell as usize)
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
    Option<&'a Wielded>,
);

/// Builds the figure from the player's body once the view exists, and
/// keeps its outfit and weapon the player's.
#[allow(clippy::too_many_arguments)]
fn dress_doll(
    mut commands: Commands,
    mut doll: ResMut<InventoryDoll>,
    players: Query<PlayerLook, With<crate::player::PlayerBody>>,
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
    let Some((outfit, body, skin, job, morph, hair, weaponry, wielded)) = players.iter().next()
    else {
        return;
    };
    let weapon = wielded.map(|w| w.model.clone());
    if let Some(figure) = scene.figure {
        if outfit.is_changed() {
            commands.entity(figure).insert(Outfit::clone(&outfit));
        }
        if weapon != scene.weapon {
            match wielded {
                Some(w) => commands.entity(figure).insert(w.clone()),
                None => commands.entity(figure).remove::<Wielded>(),
            };
            scene.weapon = weapon;
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
        Outfit::clone(&outfit),
        body.clone(),
        exa_character::WearsUnderwear,
        crate::char_view::PreviewTurn::default(),
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
    if let Some(wielded) = wielded {
        commands.entity(figure).insert(wielded.clone());
    }
    scene.figure = Some(figure);
    scene.weapon = weapon;
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
fn garment_of(item: &Item) -> Option<Garment> {
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

/// Takes a shield into the off hand, freeing it first.
pub fn hold_shield(p: &mut Possessions, id: u32, flags: u32) {
    let old = p.out[1];
    if old.id != 0 {
        possession::release_held(&mut p.carried, old.id, 1);
    }
    p.out[1] = Hand { id, flags };
}

/// Runs the character view's drop for the drops the panel input handed
/// over: the drop's plan by the item's sections, then the source panel's
/// drag end (the pack's linked view keeps its bulk), the record and the
/// player's outfit, hands and held weapon.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn equip_dropped(
    mut commands: Commands,
    mut drops: MessageReader<DollDrop>,
    mut screens: ResMut<GameScreens>,
    mut icons: ResMut<PanelIcons>,
    catalogue: Option<Res<ItemCatalogue>>,
    mut items: ResMut<DollItems>,
    game: Option<Res<Game>>,
    mut players: Query<
        (
            Entity,
            &mut Possessions,
            &mut Outfit,
            &mut Hands,
            Has<crate::torch::CarriesTorch>,
            Option<&exa_character::record::CharacterData>,
        ),
        With<crate::player::PlayerBody>,
    >,
    mut bodies: Query<(Entity, &mut Possessions), Without<crate::player::PlayerBody>>,
) {
    for drop in drops.read().copied() {
        items.ensure(game.as_deref());
        let catalogue = catalogue.as_deref();
        let kind = catalogue.and_then(|c| c.kind(drop.id)).unwrap_or_default();
        let record = items.get(drop.id).cloned();
        let has_apparel = record
            .as_ref()
            .is_some_and(|item| item.burden.is_some() && !item.worn_model.is_empty());
        let insight = drop.right_half
            && players.iter().next().is_some_and(|entry| {
                entry.5.is_some_and(|data| {
                    let skill = &data.record.skills[exa_formats::characters::INSIGHT];
                    exa_ui::character_view::insight_known(skill.points, &skill.techniques)
                })
            });
        let plan = figure_drop(DropFacts {
            id: drop.id,
            flags: drop.flags,
            from_panel: drop.source == Panel::PlayerPack,
            shown: true,
            other_panel: true,
            has_apparel,
            has_weapon: kind.weapon.is_some(),
            has_shield: kind.shield,
            race_wears_apparel: true,
            insight,
            ..DropFacts::default()
        });
        let Ok((player, mut p, mut outfit, mut hands, torch, _)) = players.single_mut() else {
            cancel(&mut screens, drop);
            continue;
        };
        // The apparel's conflict pass may refuse it (a locked piece in the
        // way); nothing moves then.
        let conflicts = match (plan.kind, record.as_ref()) {
            (FigureDrop::Apparel, Some(item)) => {
                let mut data = exa_character::record::CharacterData::of_record(
                    exa_formats::characters::CharacterRecord {
                        worn: p.worn.clone(),
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
            (FigureDrop::Weapon | FigureDrop::Shield, _) => None,
            _ => {
                cancel(&mut screens, drop);
                continue;
            }
        };
        // The source's drag ends: the inventory's pack is linked to the
        // view and keeps the item's bulk; another panel loses it.
        if let Some(container) = crate::panel_live::container_mut(&mut screens, drop.source) {
            container.finish_drag(DOLL_PANEL, false, true, 0, drop.bulk);
        }
        // The record side: the item leaves its source's list (an item
        // from the player's own pack stays listed when it goes into a
        // hand, marked held).
        let into_hand = matches!(plan.kind, FigureDrop::Weapon | FigureDrop::Shield);
        let flags = match drop.source {
            Panel::PlayerPack if into_hand => Some(drop.flags & !HELD),
            Panel::PlayerPack => body_inventory::take(&mut p, PanelKind::Pack, drop.id),
            Panel::BodyPack(body) => bodies
                .iter_mut()
                .find(|(entity, _)| entity.index().index() as i32 == body)
                .and_then(|(_, mut b)| body_inventory::take(&mut b, PanelKind::Pack, drop.id)),
            Panel::BodyEquipment(_) | Panel::Figure => None,
            Panel::Container(_) => Some(drop.flags & !HELD),
        };
        let Some(flags) = flags else {
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
            FigureDrop::Apparel => {
                let Some((removed, worn)) = conflicts else {
                    continue;
                };
                // The displaced pieces go back to the pack (the braies are
                // dropped), and the outfit loses their garments.
                for piece in &removed {
                    if let Some(item) = items.get(piece.id) {
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
                    let cell = centre(&screens);
                    screens
                        .pack
                        .add_at_centre(piece.id, icon, piece.flags, bulk);
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
                if let Some(garment) = record.as_ref().and_then(garment_of) {
                    outfit.0.push(garment);
                }
            }
            FigureDrop::Weapon | FigureDrop::Shield => {
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
                    hold_shield(&mut p, drop.id, flags);
                }
                if drop.source == Panel::PlayerPack {
                    possession::mark_held(&mut p.carried, drop.id, 1);
                } else {
                    let cell = centre(&screens);
                    body_inventory::give(
                        &mut p,
                        drop.id,
                        flags | HELD,
                        cell,
                        pack_unit,
                        pack_vscale,
                    );
                    if plan.give_to_panel {
                        screens
                            .pack
                            .add_at_centre(drop.id, drop.icon, flags | HELD, drop.bulk);
                    }
                }
                let mut next = crate::equip::hands_of(&p, |id| catalogue.and_then(|c| c.kind(id)));
                if torch {
                    next = crate::equip::keep_torch(next, &hands);
                }
                hands.set_if_neq(next);
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

/// `--doll-drop ID:FLAGS@FRAME`: the item, with its flags, put into the
/// player's pack and dropped onto the character view at that frame after
/// the load, as a drag from the pack onto the doll would (a debug tool).
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DollDropScript(pub Option<(u32, u32, u32)>);

impl DollDropScript {
    pub fn parse(text: &str) -> Option<Self> {
        let (item, frame) = text.split_once('@')?;
        let (id, flags) = item.split_once(':').unwrap_or((item, "0"));
        Some(Self(Some((
            crate::gui_drop::parse_id(id)?,
            crate::gui_drop::parse_id(flags)?,
            frame.trim().parse().ok()?,
        ))))
    }
}

/// The debug doll drop ([`DollDropScript`]).
fn scripted_doll_drop(
    script: Res<DollDropScript>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut frames: Local<u32>,
    mut players: Query<&mut Possessions, With<crate::player::PlayerBody>>,
    mut drops: MessageWriter<DollDrop>,
) {
    let Some((id, flags, frame)) = script.0 else {
        return;
    };
    if loading.is_some() || *frames > frame {
        return;
    }
    *frames += 1;
    if *frames != frame {
        return;
    }
    let Some(mut p) = players.iter_mut().next() else {
        return;
    };
    p.carried.push(possession::Carried {
        id,
        flags,
        x: 0,
        y: 0,
    });
    info!("doll drop script: {id:#x} (flags {flags:#x}) onto the doll at frame {frame}");
    drops.write(DollDrop {
        source: Panel::PlayerPack,
        id,
        flags,
        icon: 0,
        bulk: 0,
        right_half: false,
    });
}

/// A refused drop: the source's drag is cancelled and its entry stays.
fn cancel(screens: &mut GameScreens, drop: DollDrop) {
    if let Some(container) = crate::panel_live::container_mut(screens, drop.source) {
        container.finish_drag(DOLL_PANEL, true, true, 0, drop.bulk);
    }
}

/// The inventory's character view.
pub struct DollViewPlugin;

impl Plugin for DollViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryDoll>()
            .init_resource::<DollItems>()
            .init_resource::<DollDropScript>()
            .add_message::<DollDrop>()
            .add_systems(
                Update,
                (sync_doll, dress_doll, scripted_doll_drop, equip_dropped)
                    .chain()
                    .after(gui_screens::draw_screens),
            )
            .add_systems(
                PostUpdate,
                tag_doll_parts.before(exa_world::shadow_links::LightCells),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The view is the doll panel less its 2-pixel border: 224 × 380 at
    /// unit 4 for the constructor's 228 × 384 panel.
    #[test]
    fn the_view_is_the_panel_less_its_border() {
        let view = view_rect(GuiRect::new(16, 16, 228, 384), 2);
        assert_eq!((view.left, view.top), (18, 18));
        assert_eq!((view.width(), view.height()), (224, 380));
        assert_eq!(image_size((224, 380)), UVec2::new(448, 760));
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
        hold_shield(&mut p, 7, 0);
        assert_eq!(p.out[1], Hand { id: 7, flags: 0 });
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
        assert_eq!(
            DollDropScript::parse("0x1f:0x2000000@120"),
            Some(DollDropScript(Some((0x1f, 0x0200_0000, 120))))
        );
        assert_eq!(
            DollDropScript::parse("0x1f@3"),
            Some(DollDropScript(Some((0x1f, 0, 3))))
        );
        assert_eq!(DollDropScript::parse("0x1f"), None);
    }

    /// The dots draw five whole dots and a half at 5.5, 12 pixels apart at
    /// unit 4, three units down from a 28-pixel record's top.
    #[test]
    fn the_rating_dots_draw_whole_and_half_dots() {
        let cell = vec![255u8; 64 * 64 * 4];
        let sheet = DotSheet::of_cells(&[cell.clone(), cell], 64).unwrap();
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
}
