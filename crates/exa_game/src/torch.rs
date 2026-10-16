//! A lit torch carried in a hand, as the player carries one through the
//! original's dungeons. The torch is the game's own model (`torcha02`),
//! with its light and its fire, so it flickers, burns and casts its
//! shadows like any other torch; the hand holding it keeps its own handle
//! out of the light's shadow map, as the original does. Nothing keeps the
//! light off the body holding it: the original has no such rule either.
//!
//! A free main hand (the right) takes the torch, with the "Torch" overlay
//! on the one-handed moveset raising it; beside a weapon it goes in the off
//! hand (the left), with "1H & Torch". Each hand holds it by the original's
//! grip for that hand. Rules: `docs/PLAYER.md` (The carried torch).

use bevy::image::CompressedImageFormatSupport;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use exa_anim::BodyPart;
use exa_assets::{Game, ModelCache, ModelLoader, PackKind, model};
use exa_character::held::{Hands, HeldItem};
use exa_character::{RIGHT_HAND, Wielded};

pub struct TorchPlugin;

impl Plugin for TorchPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, light_torches);
    }
}

/// A body carrying a lit torch: in the right hand, or in the left beside a
/// weapon ([`TorchHand::of`]), unless a [`TorchHand`] on the body fixes it.
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
/// The weapon flags of the torch's item ("Torch A01"): a one-handed stick.
pub const TORCH_FLAGS: u16 = 0x13;

/// The hand a torch is carried in. On a body carrying a torch it fixes the
/// hand (the character view shows a torch moveset's own hand); without it
/// the hand follows [`TorchHand::of`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TorchHand {
    /// The right hand, which holds a lone item.
    Main,
    /// The left hand, beside a weapon in the right.
    Off,
}

/// How a hand holds an item: an offset (cm) and a turn (Euler angles in
/// degrees, about the hand's X, Y and Z), in the hand part's space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grip {
    pub offset: Vec3,
    pub euler: Vec3,
}

/// The original's grip for a lone item in the right hand.
pub const MAIN_GRIP: Grip = Grip {
    offset: Vec3::new(-12.0, -1.5, 6.0),
    euler: Vec3::new(70.0, -15.0, 20.0),
};
/// The original's grip for an item in the left hand.
pub const OFF_GRIP: Grip = Grip {
    offset: Vec3::new(11.0, -0.5, 6.0),
    euler: Vec3::new(110.0, 15.0, 160.0),
};

impl Grip {
    /// The item's placement under the hand part. The original's rotation
    /// builder puts the item's X axis on (cos z·cos y, sin z, −cos z·sin y):
    /// the turn about Y, then about Z, then about X, each in the frame the
    /// previous one left (glam's intrinsic `YZX`), or, applied to the item,
    /// X first, then Z, then Y, all about the hand's axes (`docs/PLAYER.md`).
    pub fn transform(self) -> Transform {
        let a = self.euler * std::f32::consts::PI / 180.0;
        let turn = Quat::from_euler(EulerRot::YZX, a.y, a.z, a.x);
        Transform::from_translation(self.offset / 100.0).with_rotation(turn)
    }
}

impl TorchHand {
    /// The right hand when it is free, the left beside a weapon.
    pub fn of(wields: bool) -> Self {
        if wields { Self::Off } else { Self::Main }
    }

    pub fn bone(self) -> usize {
        match self {
            Self::Main => RIGHT_HAND,
            Self::Off => LEFT_HAND,
        }
    }

    /// The moveset a body carrying the torch in this hand moves with, in
    /// and out of combat: the one-handed moveset with the "Torch" overlay
    /// (the right hand held up at the shoulder), or "1H & Torch" (the left),
    /// as the held items pick it (`exa_character::held`).
    pub fn moveset(self) -> &'static str {
        match self {
            Self::Main => "Torch",
            Self::Off => "1H & Torch",
        }
    }

    pub fn grip(self) -> Grip {
        match self {
            Self::Main => MAIN_GRIP,
            Self::Off => OFF_GRIP,
        }
    }

    /// The hand of a body moving with a torch moveset (`moveset`), if it is
    /// one.
    pub fn of_moveset(moveset: &str) -> Option<Self> {
        [Self::Main, Self::Off]
            .into_iter()
            .find(|h| h.moveset().eq_ignore_ascii_case(moveset))
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn light_torches(
    mut commands: Commands,
    game: Option<Res<Game>>,
    mut bodies: Query<
        (Entity, Has<Wielded>, Option<&TorchHand>, Option<&mut Hands>),
        (With<CarriesTorch>, Without<TorchLit>),
    >,
    children: Query<&Children>,
    parts: Query<&BodyPart>,
    mut cache: ResMut<ModelCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
    mut images: ResMut<Assets<Image>>,
    support: Option<Res<CompressedImageFormatSupport>>,
) {
    let Some(game) = game else { return };
    for (body, wields, fixed, hands) in &mut bodies {
        let hand = fixed.copied().unwrap_or(TorchHand::of(wields));
        let by_bone: HashMap<usize, Entity> = children
            .iter_descendants(body)
            .filter_map(|e| parts.get(e).ok().map(|p| (p.bone, e)))
            .collect();
        let Some(&holder) = by_bone.get(&hand.bone()) else {
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
        // The torch in its hand picks the moveset.
        if let Some(mut hands) = hands {
            match hand {
                TorchHand::Main => {
                    hands.main = Some(HeldItem {
                        flags: TORCH_FLAGS,
                        light: true,
                    })
                }
                TorchHand::Off => hands.off_light = true,
            }
        }
        let Some(torch) = loader.model(PackKind::Objects, TORCH) else {
            warn!("{TORCH} not found: no torch in hand");
            continue;
        };
        let grip = hand.grip().transform();
        let root = exa_world::spawn_model(&mut commands, &torch, Some(grip));
        commands.entity(root).insert((TorchRoot, ChildOf(holder)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_free_right_hand_takes_the_torch() {
        assert_eq!(TorchHand::of(false), TorchHand::Main);
        assert_eq!(TorchHand::of(false).bone(), RIGHT_HAND);
        assert_eq!(TorchHand::of(false).moveset(), "Torch");
        assert_eq!(TorchHand::of(true).bone(), LEFT_HAND);
        assert_eq!(TorchHand::of(true).moveset(), "1H & Torch");
        assert_eq!(TorchHand::of_moveset("torch"), Some(TorchHand::Main));
        assert_eq!(TorchHand::of_moveset("1H & Torch"), Some(TorchHand::Off));
        assert_eq!(TorchHand::of_moveset("One Handed"), None);
    }

    #[test]
    fn the_held_torch_picks_the_torch_hand_moveset() {
        use exa_character::held::{DEFAULT_MOVESET, choose};
        let main = Hands {
            main: Some(HeldItem {
                flags: TORCH_FLAGS,
                light: true,
            }),
            ..default()
        };
        let beside_sword = Hands {
            main: Some(HeldItem::weapon(0x11)),
            off_light: true,
            ..default()
        };
        for (hands, hand) in [(main, TorchHand::Main), (beside_sword, TorchHand::Off)] {
            assert_eq!(
                choose(&hands, DEFAULT_MOVESET, false).moveset,
                hand.moveset()
            );
        }
    }

    /// The installed torch item is a one-handed stick.
    #[test]
    fn the_shipped_torch_is_a_one_handed_stick() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let bytes = game
            .pack(PackKind::Resource)
            .and_then(|p| p.get("objdb.rdb"))
            .unwrap();
        let db = exa_formats::rdb::Database::parse(bytes).unwrap();
        let items = exa_formats::objdb::items(&db);
        let torch = items.iter().find(|i| i.name == "Torch A01").unwrap();
        assert_eq!(torch.model.to_ascii_lowercase(), TORCH);
        assert_eq!(torch.weapon.unwrap().flags & 0xFFFF, TORCH_FLAGS as u32);
    }

    #[test]
    fn the_grip_turns_about_x_then_z_then_y() {
        // The item's axes land on the rows the original builds the grip's
        // rotation from: X on (cos z·cos y, sin z, −cos z·sin y), Y on
        // (sin x·sin y − cos x·sin z·cos y, cos x·cos z,
        // sin x·cos y + cos x·sin z·sin y) and Z on
        // (cos x·sin y + sin x·sin z·cos y, −sin x·cos z,
        // cos x·cos y − sin x·sin z·sin y).
        for grip in [MAIN_GRIP, OFF_GRIP] {
            let a = grip.euler * std::f32::consts::PI / 180.0;
            let (sx, cx) = a.x.sin_cos();
            let (sy, cy) = a.y.sin_cos();
            let (sz, cz) = a.z.sin_cos();
            let q = grip.transform().rotation;
            let x = Vec3::new(cz * cy, sz, -cz * sy);
            let y = Vec3::new(sx * sy - cx * sz * cy, cx * cz, sx * cy + cx * sz * sy);
            let z = Vec3::new(cx * sy + sx * sz * cy, -sx * cz, cx * cy - sx * sz * sy);
            assert!((q * Vec3::X - x).length() < 1e-5, "{}", q * Vec3::X);
            assert!((q * Vec3::Y - y).length() < 1e-5, "{}", q * Vec3::Y);
            assert!((q * Vec3::Z - z).length() < 1e-5, "{}", q * Vec3::Z);
        }
    }

    /// How a clip holds the right arm, from its first key on the body
    /// model's bone offsets, in the chest's frame (+X the body's left, +Y
    /// up, +Z forward).
    #[derive(Debug)]
    struct RightArm {
        /// The angle between the upper arm and the forearm (°): 0 straight.
        elbow: f32,
        /// The upper arm's tilt out to the side and forward from hanging
        /// straight down (°).
        out: f32,
        forward: f32,
        /// The wrist (the hand's origin) from the shoulder joint (the upper
        /// arm's origin), cm.
        wrist: Vec3,
    }

    const PELVIS: usize = 1;
    const CHEST: usize = 9;
    const UPPER_ARM: usize = 12;
    const FOREARM: usize = 13;

    /// Each bone's parent bone and offset in it (m), from `humanbase.rfc`.
    fn bone_offsets(scene: &exa_formats::rfc::Scene) -> HashMap<usize, (Option<usize>, Vec3)> {
        let bone_of = |i: usize| exa_formats::rsq::bone_index(&scene.nodes[i].name);
        (0..scene.nodes.len())
            .filter_map(|n| {
                let bone = bone_of(n)?;
                let mut up = scene.nodes[n].parent.map(|p| p - 1);
                while let Some(u) = up.filter(|&u| bone_of(u).is_none()) {
                    up = scene.nodes[u].parent.map(|p| p - 1);
                }
                let parent = up.and_then(bone_of);
                Some((bone, (parent, Vec3::from(scene.nodes[n].transform[3]))))
            })
            .collect()
    }

    fn right_arm(
        motions: &exa_formats::rsq::MotionSet,
        offsets: &HashMap<usize, (Option<usize>, Vec3)>,
        moveset: &str,
        action: &str,
    ) -> RightArm {
        let clip = motions.clip(moveset, action).expect("clip");
        let key = &clip.keys[0];
        let mut world: HashMap<usize, (Quat, Vec3)> = HashMap::default();
        // Every bone's parent has a lower number.
        for bone in PELVIS..=LEFT_HAND {
            let r = key.rotations[bone - 1];
            let turn = Quat::from_xyzw(r[0], r[1], r[2], r[3]).normalize();
            let (parent, offset) = offsets[&bone];
            let placed = match parent {
                None => (turn, Vec3::ZERO),
                Some(p) => {
                    let (pr, pt) = world[&p];
                    (pr * turn, pt + pr * offset)
                }
            };
            world.insert(bone, placed);
        }
        let chest = world[&CHEST].0.inverse();
        let at = |bone: usize| world[&bone].1;
        let upper = chest * (at(FOREARM) - at(UPPER_ARM));
        let fore = chest * (at(RIGHT_HAND) - at(FOREARM));
        RightArm {
            elbow: upper.angle_between(fore).to_degrees(),
            out: (-upper.x).atan2(-upper.y).to_degrees(),
            forward: upper.z.atan2(-upper.y).to_degrees(),
            wrist: chest * (at(RIGHT_HAND) - at(UPPER_ARM)) * 100.0,
        }
    }

    /// The installed "Torch" clips, on the body model: out of combat the
    /// torch is held up beside the head, the elbow bent to its 120° limit
    /// and the upper arm 40° out to the side; the combat idle is the
    /// compact carry of the user's reference shots of the original, the
    /// upper arm tucked and forward, the elbow at about 100°, the wrist
    /// before the chest. "Torch" overlays whole clips on "One Handed"
    /// (Walk, Cmbt - Idle, Attack Mid and Idle); the guard is One
    /// Handed's.
    #[test]
    fn the_torch_clips_raise_it_beside_the_head_and_carry_it_before_the_chest() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let data = std::sync::Arc::new(exa_assets::GameData::open(&dir).unwrap());
        let motions =
            exa_anim::Motions::load(&exa_assets::Game(data.clone())).expect("humanoid.rsq");
        let set = &motions.0;
        let bytes = data
            .pack(PackKind::Resource)
            .and_then(|p| p.get(model::BODY_MODEL))
            .expect("humanbase.rfc");
        let offsets = bone_offsets(&exa_formats::rfc::Scene::parse(bytes).unwrap());
        assert_eq!(offsets.len(), 18);

        let torch = set.moveset("Torch").unwrap();
        assert_eq!(torch.parent, "One Handed");
        let defined: Vec<&str> = set
            .actions
            .iter()
            .zip(&torch.clips)
            .filter(|(_, c)| c.is_some())
            .map(|(a, _)| a.name.as_str())
            .collect();
        assert_eq!(defined, ["Walk", "Cmbt - Idle", "Attack Mid", "Idle"]);
        assert_eq!(
            set.clip("Torch", "Defence stance"),
            set.clip("One Handed", "Defence stance")
        );

        let near = |got: f32, want: f32, what: &str| {
            assert!((got - want).abs() < 0.1, "{what}: {got:.2}, not {want}");
        };
        let idle = right_arm(set, &offsets, "Torch", "Idle");
        near(idle.elbow, 122.6, "Idle elbow");
        near(idle.out, 39.8, "Idle upper arm out");
        near(idle.forward, 11.4, "Idle upper arm forward");
        assert!(
            idle.wrist.abs_diff_eq(Vec3::new(-16.4, 1.4, 18.7), 0.1),
            "Idle wrist {}",
            idle.wrist
        );
        let combat = right_arm(set, &offsets, "Torch", "Cmbt - Idle");
        near(combat.elbow, 101.5, "Cmbt - Idle elbow");
        near(combat.out, 24.9, "Cmbt - Idle upper arm out");
        near(combat.forward, 31.8, "Cmbt - Idle upper arm forward");
        assert!(
            combat.wrist.abs_diff_eq(Vec3::new(-8.6, -3.0, 31.6), 0.1),
            "Cmbt - Idle wrist {}",
            combat.wrist
        );
    }

    #[test]
    fn main_grip_places_the_torch_in_the_fist() {
        let t = MAIN_GRIP.transform();
        assert!((t.translation - Vec3::new(-0.12, -0.015, 0.06)).length() < 1e-6);
        // The handle (the model's +Y) runs mostly along the hand's Z, the
        // axis the fist closes around.
        let handle = t.rotation * Vec3::Y;
        assert!(handle.z.abs() > 0.7, "{handle}");
    }
}
