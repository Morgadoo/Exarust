//! The character creator's preview: the character being made, standing on
//! the creation scene's floor, drawn by a 3D view of its own in the
//! creator form's preview viewport.
//!
//! The view is the inventory doll's ([`crate::doll_view`]) rebuilt around
//! the creator's record instead of the player: an off-screen camera at the
//! preview's eye (−200, 210, 360) cm looking at (0, 60, 0) cm with a 50°
//! field of view ([`crate::create::preview_view`] and its constants) draws
//! the floor scene `sgccflr01.rfc` and the figure into an image the size
//! of the viewport, which a UI node shows under the GUI canvas; the
//! creator's painter clears the canvas over the viewport so the view shows
//! through. The figure is the record's body: its race's model morphed by
//! the record's physique, its skin look and generated skin maps, its hair
//! and the underwear the creator dresses it in, idling and turning toward
//! the pointer ([`crate::char_view`]). A change of the record rebuilds the
//! figure once the creator's regeneration rule lets it
//! ([`crate::creator_screen::Creator::take_regeneration`]).
//!
//! The floor scene's own two lights (a white key light in front, a warm one
//! behind) light the view once it is loaded; until then the doll's view
//! light stands in.
//!
//! Guesses: the field of view is taken as vertical, as the doll's; the
//! view's fog (400, 0.34) and the preview's mesh smoothing of the hair are
//! not applied.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::image::CompressedImageFormatSupport;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::window::PrimaryWindow;
use exa_assets::model::{self, ModelLoader};
use exa_assets::{Game, ModelCache, PackKind};
use exa_formats::characters::CharacterRecord;
use exa_ui::gui_core::GuiRect;

use crate::create::{PREVIEW_EYE_CM, PREVIEW_FOV_DEG, PREVIEW_SCENE, PREVIEW_TARGET_CM};
use crate::creator_screen::CreatorScreen;

/// The render layer of the preview's camera, light, floor and figure: one
/// above the inventory doll's, so neither view sees the other. Exarust's
/// choice.
pub const CREATOR_LAYER: usize = crate::doll_view::DOLL_LAYER + 1;
/// Where the preview's scene stands, metres: below the doll's, far from
/// any level.
pub const CREATOR_ORIGIN: Vec3 = Vec3::new(0.0, -2000.0, 0.0);
/// The clear colour behind the floor: the scene's fog fades to black
/// (fog mode 2).
pub const CREATOR_CLEAR: Color = Color::BLACK;

/// The view's render layers.
pub fn creator_layers() -> RenderLayers {
    RenderLayers::from_layers(&[CREATOR_LAYER])
}

/// The preview camera's transform: the eye and target converted from
/// centimetres around the scene's origin, looking at the target.
pub fn camera_transform() -> Transform {
    let eye = Vec3::from(PREVIEW_EYE_CM) * 0.01 + CREATOR_ORIGIN;
    let target = Vec3::from(PREVIEW_TARGET_CM) * 0.01 + CREATOR_ORIGIN;
    Transform::from_translation(eye).looking_at(target, Vec3::Y)
}

/// The preview's camera, image, light, node, floor and figure.
pub struct CreatorScene {
    pub image: Handle<Image>,
    pub camera: Entity,
    pub light: Entity,
    pub node: Entity,
    pub floor: Option<Entity>,
    pub floor_tried: bool,
    /// The floor scene brought its own lights.
    pub scene_lit: bool,
    pub figure: Option<Entity>,
    /// The viewport the scene was made for, in window pixels.
    pub view: GuiRect,
    /// The hub loadout preview's revision the figure was built from.
    pub kit_revision: u64,
    /// The record the figure was last built or re-textured from, which a
    /// change is compared with to pick the rebuild path
    /// ([`exa_character::rebuild::preview_change`]).
    pub built: Option<CharacterRecord>,
}

/// The preview's state while the creator is open.
#[derive(Resource, Default)]
pub struct CreatorView {
    pub scene: Option<CreatorScene>,
}

/// Marks a mesh moved onto the preview's layer.
#[derive(Component)]
struct CreatorPart;

pub struct CreatorViewPlugin;

impl Plugin for CreatorViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CreatorView>()
            .add_systems(
                Update,
                (sync_view, script_edits, build_figure)
                    .chain()
                    .after(crate::creator_screen::CreatorSystems),
            )
            .add_systems(
                PostUpdate,
                tag_parts.before(exa_world::shadow_links::LightCells),
            );
    }
}

fn despawn_scene(commands: &mut Commands, scene: CreatorScene) {
    for entity in [scene.camera, scene.light, scene.node]
        .into_iter()
        .chain(scene.floor)
        .chain(scene.figure)
    {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.despawn();
        }
    }
}

fn spawn_scene(commands: &mut Commands, images: &mut Assets<Image>, view: GuiRect) -> CreatorScene {
    let size = crate::doll_view::image_size((view.width(), view.height()));
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
    let layer = creator_layers();
    let camera_at = camera_transform();
    let camera = commands
        .spawn((
            Name::new("_CreatorCamera"),
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(CREATOR_CLEAR),
                ..default()
            },
            RenderTarget::Image(image.clone().into()),
            Projection::Perspective(PerspectiveProjection {
                fov: PREVIEW_FOV_DEG.to_radians(),
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
            Name::new("_CreatorLight"),
            DirectionalLight {
                color: Color::WHITE,
                illuminance: crate::doll_view::DOLL_SUN,
                shadow_maps_enabled: false,
                ..default()
            },
            crate::doll_view::light_transform(&camera_at),
            layer,
            exa_render::supersample::Offscreen,
        ))
        .id();
    let node = commands
        .spawn((
            Name::new("_CreatorView"),
            ImageNode {
                image: image.clone(),
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            GlobalZIndex(crate::doll_view::DOLL_Z),
        ))
        .id();
    CreatorScene {
        image,
        camera,
        light,
        node,
        floor: None,
        floor_tried: false,
        scene_lit: false,
        figure: None,
        view,
        kit_revision: 0,
        built: None,
    }
}

/// The view follows the creator: made when it opens (and again when the
/// viewport's size changes), placed over the viewport, the pointer handed
/// to the figure's turn, and everything dropped when it closes.
#[allow(clippy::too_many_arguments)]
fn sync_view(
    mut commands: Commands,
    screen: Res<CreatorScreen>,
    mut view: ResMut<CreatorView>,
    mut images: ResMut<Assets<Image>>,
    mut pointer: ResMut<crate::char_view::PreviewPointer>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut nodes: Query<&mut Node>,
    kit: Option<Res<crate::loadout_kit::KitPreview>>,
) {
    // The creator's viewport, else the hub loadout form's preview (the
    // form reuses the creator's view helper).
    let rect = screen
        .preview_px()
        .or_else(|| kit.as_ref().and_then(|kit| kit.rect));
    let Some(rect) = rect else {
        if let Some(scene) = view.scene.take() {
            despawn_scene(&mut commands, scene);
            pointer.0 = None;
        }
        return;
    };
    if rect.width() <= 0 || rect.height() <= 0 {
        return;
    }
    let resized = view.scene.as_ref().is_some_and(|scene| {
        (scene.view.width(), scene.view.height()) != (rect.width(), rect.height())
    });
    if resized && let Some(scene) = view.scene.take() {
        despawn_scene(&mut commands, scene);
    }
    let scene = view
        .scene
        .get_or_insert_with(|| spawn_scene(&mut commands, &mut images, rect));
    scene.view = rect;
    let Ok(window) = windows.single() else {
        return;
    };
    let scale = window.scale_factor();
    if let Ok(mut node) = nodes.get_mut(scene.node) {
        node.left = Val::Px(rect.left as f32 / scale);
        node.top = Val::Px(rect.top as f32 / scale);
        node.width = Val::Px(rect.width() as f32 / scale);
        node.height = Val::Px(rect.height() as f32 / scale);
    }
    pointer.0 = window.cursor_position().map(|cursor| {
        let at = cursor * scale - Vec2::new(rect.left as f32, rect.top as f32);
        crate::char_view::pointer_point(
            &camera_transform(),
            PREVIEW_FOV_DEG.to_radians(),
            CREATOR_ORIGIN,
            at,
            rect.width() as f32,
            rect.height() as f32,
        )
    });
}

/// Builds the floor once, and the figure from the creator's record when it
/// is missing or the record changed and the regeneration rule lets it.
#[allow(clippy::too_many_arguments)]
fn build_figure(
    mut commands: Commands,
    mut screen: ResMut<CreatorScreen>,
    mut view: ResMut<CreatorView>,
    game: Option<Res<Game>>,
    cache: Option<ResMut<ModelCache>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<exa_render::material::ExaMaterial>>>,
    images: Option<ResMut<Assets<Image>>>,
    support: Option<Res<CompressedImageFormatSupport>>,
    (kit, mut items): (
        Option<Res<crate::loadout_kit::KitPreview>>,
        Option<ResMut<crate::doll_view::DollItems>>,
    ),
    figures: FigureState,
) {
    let Some(scene) = view.scene.as_mut() else {
        return;
    };
    // The hub loadout preview's record wears its kit: its worn items as
    // the figure's outfit.
    let mut outfit = None;
    let (regenerate, record) = match screen.preview_record() {
        Some((regenerate, record)) => (regenerate, record.clone()),
        None => {
            let Some(kit) = kit.as_deref() else {
                return;
            };
            let Some(record) = kit.record.clone() else {
                return;
            };
            let changed = scene.kit_revision != kit.revision;
            scene.kit_revision = kit.revision;
            if let Some(items) = items.as_deref_mut() {
                items.ensure(game.as_deref());
                outfit = Some(exa_character::Outfit(
                    record
                        .worn
                        .iter()
                        .filter_map(|worn| items.get(worn.id))
                        .filter_map(crate::doll_view::garment_of)
                        .collect(),
                ));
            }
            (changed, record)
        }
    };
    let record = &record;
    let rebuild = scene.figure.is_none() || regenerate;
    if !rebuild && scene.floor_tried {
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
    if !scene.floor_tried {
        scene.floor_tried = true;
        match loader.model(PackKind::Resource, PREVIEW_SCENE) {
            Some(floor) => {
                let entity = exa_world::spawn_model(&mut commands, &floor, None);
                commands.entity(entity).insert((
                    Name::new("_CreatorFloor"),
                    Transform::from_translation(CREATOR_ORIGIN),
                    Visibility::default(),
                ));
                scene.floor = Some(entity);
            }
            None => warn!("creator: the preview floor {PREVIEW_SCENE} is missing"),
        }
    }
    if !rebuild {
        return;
    }
    // A creator edit takes the path its control calls: a re-texture or a
    // hair rebuild keeps the figure; the whole rebuild builds it again in
    // the pose it stood in.
    let change = match (&scene.built, scene.figure) {
        (Some(built), Some(_)) if outfit.is_none() => {
            exa_character::rebuild::preview_change(built, record)
        }
        _ => exa_character::rebuild::PreviewChange {
            rebuild: true,
            ..Default::default()
        },
    };
    if let Some(figure) = scene.figure.filter(|_| !change.rebuild) {
        renew_in_place(&mut commands, &game, record, figure, change, &figures);
        scene.built = Some(record.clone());
        return;
    }
    // The whole rebuild saves the old body's motion and turn and gives
    // them to the new one, as the original keeps the character's frames
    // across its rebuild.
    let kept = scene
        .figure
        .and_then(|old| figures.kept.get(old).ok())
        .map(|(animator, turn)| (animator.cloned(), turn.copied()));
    if let Some(old) = scene.figure.take()
        && let Ok(mut e) = commands.get_entity(old)
    {
        e.despawn();
    }
    scene.figure = spawn_figure(&mut commands, &mut loader, &game, record);
    scene.built = scene.figure.map(|_| record.clone());
    if let (Some(figure), Some((animator, turn))) = (scene.figure, kept) {
        let mut e = commands.entity(figure);
        if let Some(animator) = animator {
            e.insert(animator);
        }
        if let Some(turn) = turn {
            e.insert(turn);
        }
    }
    if let (Some(figure), Some(outfit)) = (scene.figure, outfit) {
        info!(
            "loadout preview: {} (race {}) wears {} garments, figure {figure:?}",
            record.name,
            record.race,
            outfit.0.len()
        );
        commands.entity(figure).insert(outfit);
    }
}

/// A debug edit script for the preview (`EXA_CREATOR_EDITS=1`): frames
/// after the creator opens it changes the skin tone (a re-texture), the
/// hair style (a hair rebuild) and the physique (the whole rebuild), one
/// at a time, so a run shows each path in the log.
fn script_edits(mut screen: ResMut<CreatorScreen>, mut frame: Local<u32>) {
    if std::env::var_os("EXA_CREATOR_EDITS").is_none() {
        return;
    }
    let Some(creator) = screen.creator.as_mut() else {
        *frame = 0;
        return;
    };
    *frame += 1;
    match *frame {
        120 => creator.edit_record(|r| r.skin = [0x2000, 0xC000]),
        240 => creator.edit_record(|r| r.hair_style = r.hair_style % 8 + 1),
        360 => creator.edit_record(|r| r.hair_colour = [200, 40, 120, r.hair_colour[3]]),
        480 => creator.edit_record(|r| r.physique = [0.9, -0.6]),
        _ => return,
    }
    info!("creator edits: frame {} edit sent", *frame);
}

/// What the narrow rebuild paths read and change on a built figure.
#[derive(bevy::ecs::system::SystemParam)]
pub struct FigureState<'w, 's> {
    kept: Query<
        'w,
        's,
        (
            Option<&'static exa_anim::Animator>,
            Option<&'static crate::char_view::PreviewTurn>,
        ),
    >,
    hair: Query<'w, 's, &'static exa_character::hair::Hair>,
    children: Query<'w, 's, &'static Children>,
    hair_roots: Query<'w, 's, (), With<exa_character::hair::HairRoot>>,
}

/// The race of `record` from the game's race table.
fn race_of(game: &Game, record: &CharacterRecord) -> Option<exa_formats::races::Race> {
    let races = game
        .0
        .pack(PackKind::Resource)
        .and_then(|p| p.get("races.rdb"))
        .and_then(|b| exa_formats::races::parse(b).ok())
        .unwrap_or_default();
    exa_formats::races::of_character(&races, record.race).cloned()
}

/// The narrow rebuild paths on the built `figure`: the skin job built again
/// from `record` (the texture-only rebuild), and the hair removed and grown
/// again (the hair rebuild: nothing grows again when the race has no hair,
/// [`exa_character::rebuild::hair_rebuild`]). A hair-colour edit also
/// grows the hair again, as Exarust's hair parts carry the colour in their
/// own materials (the original's route from the colour picker to the
/// scalp's material is not traced; its re-texture repaints the beard).
fn renew_in_place(
    commands: &mut Commands,
    game: &Game,
    record: &CharacterRecord,
    figure: Entity,
    change: exa_character::rebuild::PreviewChange,
    figures: &FigureState,
) {
    let race = race_of(game, record);
    let look_word = race.as_ref().map_or(0, |r| r.look);
    if change.retexture {
        let model_name = race
            .as_ref()
            .map_or_else(|| model::BODY_MODEL.to_string(), |r| r.model());
        let physique = exa_formats::morph::Physique::of_record(record);
        let (look, job) = crate::scene_bodies::character_retexture(
            &game.0,
            &model_name,
            record,
            look_word,
            &physique,
            record.id ^ FIGURE_SEED,
            &[],
        );
        let mut e = commands.entity(figure);
        e.insert(look);
        match job {
            Some(job) => e.insert(job),
            None => e.remove::<exa_assets::SkinCompositeJob>(),
        };
    }
    let mut hair = hair_of(record, race.as_ref());
    let old = figures.hair.get(figure).ok().copied();
    if let Some(old) = old {
        hair.head_word = old.head_word;
    }
    if !change.hair && old == Some(hair) {
        return;
    }
    let decision =
        exa_character::rebuild::hair_rebuild(std::iter::empty(), look_word, hair.head_word);
    if decision.remove_only {
        hair.style = 0;
    }
    for part in figures.children.iter_descendants(figure) {
        if figures.hair_roots.contains(part) {
            commands.entity(part).despawn();
        }
    }
    commands
        .entity(figure)
        .insert(hair)
        .remove::<exa_character::hair::HairAttached>();
    info!(
        "creator: figure {figure:?} renewed in place (re-texture {}, hair {})",
        change.retexture, change.hair
    );
}

/// The seed the figure's skin job draws a fresh look seed from.
const FIGURE_SEED: u32 = 0x51ED_270B;

/// The figure of `record`: its race's body morphed by its physique, its
/// skin, hair and the underwear, idling at the scene's origin.
pub fn spawn_figure(
    commands: &mut Commands,
    loader: &mut ModelLoader,
    game: &Game,
    record: &CharacterRecord,
) -> Option<Entity> {
    let races = game
        .0
        .pack(PackKind::Resource)
        .and_then(|p| p.get("races.rdb"))
        .and_then(|b| exa_formats::races::parse(b).ok())
        .unwrap_or_default();
    let race = exa_formats::races::of_character(&races, record.race);
    let model_name = race.map_or_else(|| model::BODY_MODEL.to_string(), |r| r.model());
    let moveset = race.map_or("Default", |r| r.moveset()).to_string();
    let morph = exa_assets::BodyMorph::of_model(
        &model_name,
        exa_formats::morph::Physique::of_record(record),
    );
    let body = loader.morphed_model(PackKind::Resource, &model_name, Some(&morph))?;
    let (look, job) = crate::scene_bodies::character_skin(
        &game.0,
        &model_name,
        record,
        race.map_or(0, |r| r.look),
        &morph.physique,
        record.id ^ FIGURE_SEED,
    );
    let figure = exa_world::spawn_model(commands, &body, None);
    commands.entity(figure).insert((
        Name::new("_CreatorFigure"),
        Transform::from_translation(CREATOR_ORIGIN),
        Visibility::default(),
        exa_anim::Animator::new(&moveset, "Idle"),
        exa_anim::FrameClips,
        exa_character::Outfit::default(),
        exa_character::WearsUnderwear,
        exa_assets::BodyModel(model_name),
        morph,
        look,
        hair_of(record, race),
        crate::char_view::PreviewTurn::default(),
    ));
    if let Some(job) = job {
        commands.entity(figure).insert(job);
    }
    Some(figure)
}

/// The record's hair, as the level's characters grow it: only a race whose
/// look word has hair grows the record's style.
pub fn hair_of(
    record: &CharacterRecord,
    race: Option<&exa_formats::races::Race>,
) -> exa_character::hair::Hair {
    exa_character::hair::Hair {
        style: if race.is_none_or(|r| r.look & exa_formats::races::LOOK_HAIR != 0) {
            record.hair_style
        } else {
            0
        },
        colour: [
            record.hair_colour[0],
            record.hair_colour[1],
            record.hair_colour[2],
        ],
        feral: exa_character::look::build_feral(record),
        light_variant: record.profession_rest & 0x1_0000 != 0,
        head_word: 0,
    }
}

/// Moves every mesh of the floor and figure onto the preview's layer, out
/// of the shadow links, and the floor scene's own lights with them; once
/// the scene brings its lights the stand-in light goes.
#[allow(clippy::type_complexity)]
fn tag_parts(
    mut commands: Commands,
    mut view: ResMut<CreatorView>,
    children: Query<&Children>,
    meshes: Query<(), (With<Mesh3d>, Without<CreatorPart>)>,
    lights: Query<
        (),
        (
            Or<(With<PointLight>, With<SpotLight>)>,
            Without<CreatorPart>,
        ),
    >,
) {
    let Some(scene) = view.scene.as_mut() else {
        return;
    };
    let layer = creator_layers();
    if let Some(floor) = scene.floor {
        let mut lit = false;
        for entity in std::iter::once(floor).chain(children.iter_descendants(floor)) {
            if lights.contains(entity) {
                commands.entity(entity).insert((layer.clone(), CreatorPart));
                lit = true;
            }
        }
        // The hub loadout preview keeps its stand-in light: in a level the
        // floor scene's lights do not reach the view [deviation: the
        // level's light list leaves them out].
        if lit
            && !scene.scene_lit
            && scene.kit_revision == 0
            && let Ok(mut e) = commands.get_entity(scene.light)
        {
            e.despawn();
            scene.scene_lit = true;
        }
    }
    for root in scene.floor.into_iter().chain(scene.figure) {
        for entity in std::iter::once(root).chain(children.iter_descendants(root)) {
            if meshes.contains(entity) {
                commands.entity(entity).insert((
                    layer.clone(),
                    NotShadowCaster,
                    NotShadowReceiver,
                    CreatorPart,
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_camera_looks_from_the_preview_eye_at_its_target() {
        let at = camera_transform();
        let eye = Vec3::new(-2.0, 2.1, 3.6) + CREATOR_ORIGIN;
        assert!((at.translation - eye).length() < 1e-4);
        let toward = (Vec3::new(0.0, 0.6, 0.0) - Vec3::new(-2.0, 2.1, 3.6)).normalize();
        assert!((at.forward().as_vec3() - toward).length() < 1e-5);
    }

    #[test]
    fn the_hair_takes_the_records_style_and_colour() {
        let record = CharacterRecord {
            hair_style: 3,
            hair_colour: [10, 20, 30, 0],
            ..CharacterRecord::default()
        };
        let hair = hair_of(&record, None);
        assert_eq!((hair.style, hair.colour), (3, [10, 20, 30]));
        assert!(!hair.light_variant);
    }
}
