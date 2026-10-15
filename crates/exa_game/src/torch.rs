//! A lit torch carried in the left hand, as the player carries one through
//! the original's dungeons. The torch is the game's own model (`torcha02`),
//! with its light and its fire, so it flickers, burns and casts its
//! shadows like any other torch; the hand holding it keeps its own handle
//! out of the light's shadow map, as the original does.

use bevy::image::CompressedImageFormatSupport;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use exa_anim::BodyPart;
use exa_assets::{Game, ModelCache, ModelLoader, PackKind, model};

pub struct TorchPlugin;

impl Plugin for TorchPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, light_torches);
    }
}

/// A body carrying a lit torch in its left hand.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct CarriesTorch;

/// The torch is in the hand.
#[derive(Component)]
struct TorchLit;

/// The root of a carried torch's model.
#[derive(Component)]
pub struct TorchRoot;

/// Left hand bone (the right is `exa_character::RIGHT_HAND`).
pub const LEFT_HAND: usize = 18;
/// The torch model in `Objects.rpk`.
const TORCH: &str = "torcha02";

/// The torch upright in the fist, its flame above the hand: the model's
/// handle runs along its +Y, turned onto the left hand's grip.
fn left_grip() -> Transform {
    Transform::from_xyz(0.085, -0.01, 0.0)
        .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))
}

#[allow(clippy::too_many_arguments)]
fn light_torches(
    mut commands: Commands,
    game: Option<Res<Game>>,
    bodies: Query<Entity, (With<CarriesTorch>, Without<TorchLit>)>,
    children: Query<&Children>,
    parts: Query<&BodyPart>,
    mut cache: ResMut<ModelCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
    mut images: ResMut<Assets<Image>>,
    support: Option<Res<CompressedImageFormatSupport>>,
) {
    let Some(game) = game else { return };
    for body in &bodies {
        let by_bone: HashMap<usize, Entity> = children
            .iter_descendants(body)
            .filter_map(|e| parts.get(e).ok().map(|p| (p.bone, e)))
            .collect();
        let Some(&hand) = by_bone.get(&LEFT_HAND) else {
            continue; // body parts not attached yet
        };
        let mut loader = ModelLoader {
            game: &game.0,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: model::bc_supported(support.as_deref()),
        };
        commands.entity(body).insert(TorchLit);
        let Some(torch) = loader.model(PackKind::Objects, TORCH) else {
            warn!("{TORCH} not found: no torch in hand");
            continue;
        };
        let root = exa_world::spawn_model(&mut commands, &torch, Some(left_grip()));
        commands.entity(root).insert((TorchRoot, ChildOf(hand)));
    }
}
