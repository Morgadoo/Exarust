//! The Force and Displacement effects' scene side: the materials, shader
//! parameters, animated noise and GI programs their creators attach, the
//! creators' node positions, attachments and sounds, the effect shader's
//! scrolling texture passes, the body's fade-hook list, the decorative
//! shell, the tendril geometry gathered from a body's bones, and the
//! teleport placement through the physics body's place-at query.
//!
//! The creators build a [`SceneNodeSpec`] from the caster's hand and body,
//! the effect's point and the power's numbers; [`VisualPlugin`] turns a
//! [`VisualSpawn`] message into a scene entity carrying [`EffectVisual`].
//! The entity has no mesh yet (the effect geometries are their own classes),
//! so it marks the node for the renderer rather than drawing it.
//!
//! Evidence: the Force and Displacement creators, their materials and the
//! shared effect shader binding, the two per-pass texture-matrix callbacks,
//! the fade hook list, the decorative shell and the body placement query.

use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_physics::body::Ragdoll;

use super::displacement::wisp_anchors;

/// The blast family's material.
pub const FORCE_MATERIAL: &str = "_ForceMat01";
/// The domes' and the barricade's material.
pub const BARRIER_MATERIAL: &str = "_ForceBarrier01";
/// The shell's material.
pub const AURA_MATERIAL: &str = "_ForceAura01";
/// The tendril's material.
pub const TENDRIL_MATERIAL: &str = "_TDispMat01";
/// The displacement shell's material.
pub const SHELL_MATERIAL: &str = "_EffectShell01";
/// The Force noise picture.
pub const NOISE_TEXTURE: &str = "_ForceNoise";
/// The shared emissive program.
pub const GI_PROGRAM: &str = "gi_multipoint";
/// Its light and attenuation bindings.
pub const GI_LIGHT: &str = "Light";
pub const GI_ATTEN: &str = "Atten";

/// The material parameters each of the three Force materials binds.
pub const MATERIAL_PARAMS: [f32; 4] = [0.25, 0.25, 0.25, 1.0];
/// The blast material's parameter block.
pub const FORCE_BLOCK: usize = 0xf0;
/// The barrier material's parameter block.
pub const BARRIER_BLOCK: usize = 0xb8;
/// The aura material's parameter block.
pub const AURA_BLOCK: usize = 0xf0;
/// The tendril and shell materials' parameter block.
pub const DISPLACEMENT_BLOCK: usize = 0xf0;

/// The noise picture's size and channels.
pub const NOISE_SIZE: (u32, u32, u32) = (256, 256, 4);
/// The noise picture's layers.
pub const NOISE_LAYERS: u32 = 0x33;
/// The noise texture's sampler word.
pub const NOISE_FLAGS: u32 = 0x0100_4200;
/// The noise texture's usage word.
pub const NOISE_USAGE: u32 = 0x0200_7000;

/// The node flags every Force effect node starts with.
pub const EFFECT_NODE: u32 = 0x2100_0000;
/// The shell subobject's flag.
pub const SHELL_FLAG: u32 = 0x20;
/// The decorative shell's two extra fields.
pub const DECORATIVE_FLAG: u8 = 1;
pub const DECORATIVE_SCALE: f32 = 3.0;
/// The spare strands a tendril image keeps.
pub const TENDRIL_SPARE: usize = 8;
/// The tendril texture.
pub const TENDRIL_TEXTURE: &str = "FlameletsA3.rfi";
/// The tendril geometry's registered type word.
pub const TENDRIL_KIND: u32 = 0x15;

/// One creator's scene node.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneNodeSpec {
    /// The node's material, if it makes one.
    pub material: Option<&'static str>,
    /// The node's flags.
    pub flags: u32,
    /// The shared emissive program.
    pub gi: Option<&'static str>,
    /// The material's parameter block size.
    pub block: usize,
    /// The node's place (cm).
    pub position: Vec3,
    /// The node's orientation.
    pub rotation: Quat,
    /// The node is attached to the caster's hand.
    pub hand: bool,
    /// The sounds the creator starts, with their pitch.
    pub sounds: Vec<(&'static str, f32)>,
    /// One radio of the effect's own lifetime, when it has one.
    pub life_ms: u32,
}

impl Default for SceneNodeSpec {
    fn default() -> Self {
        Self {
            material: None,
            flags: EFFECT_NODE,
            gi: None,
            block: 0,
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            hand: false,
            sounds: Vec::new(),
            life_ms: 0,
        }
    }
}

/// What a creator makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectKind {
    Blast,
    Bolt,
    Wave,
    Torrent,
    Barrier,
    Bubble,
    Barricade,
    Shell,
    DecorativeShell,
    Vanish,
    LongDoor,
    Constant,
    Summon,
    Tendril,
    Kinesis,
    Throw,
    Strikes,
}

/// A creator's place and numbers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatorInput {
    /// The caster's body (its feet).
    pub body: Vec3,
    /// The caster's heading.
    pub heading: Vec3,
    /// The caster's hand.
    pub hand: Vec3,
    /// The effect's point, when it has one.
    pub point: Option<Vec3>,
    /// The random draw for a pitch (`0` to `1`).
    pub roll: f32,
}

/// Builds a creator's scene node.
pub fn creator(kind: EffectKind, input: &CreatorInput) -> SceneNodeSpec {
    let hand = |material, sound| SceneNodeSpec {
        material: Some(material),
        gi: Some(GI_PROGRAM),
        block: FORCE_BLOCK,
        position: input.hand,
        hand: true,
        sounds: vec![sound],
        ..SceneNodeSpec::default()
    };
    match kind {
        EffectKind::Blast => hand(FORCE_MATERIAL, ("thmfrc_cu01", 1.0)),
        EffectKind::Bolt => SceneNodeSpec {
            sounds: vec![("thmfrc_blt", 1.0)],
            ..hand(FORCE_MATERIAL, ("thmfrc_cu01", 1.0))
        },
        EffectKind::Wave => SceneNodeSpec {
            position: input.body,
            hand: false,
            sounds: vec![("thmfrc_wav", 1.0), ("thmfrc_cu01", 1.0)],
            ..hand(FORCE_MATERIAL, ("thmfrc_cu01", 1.0))
        },
        EffectKind::Torrent => SceneNodeSpec {
            sounds: vec![("thmfrc_cu01", 1.0), ("thmfrc_trntdr", 1.0)],
            ..hand(FORCE_MATERIAL, ("thmfrc_cu01", 1.0))
        },
        EffectKind::Barrier => SceneNodeSpec {
            material: Some(BARRIER_MATERIAL),
            gi: Some(GI_PROGRAM),
            block: BARRIER_BLOCK,
            position: input.body + Vec3::Y * 100.0,
            rotation: Quat::from_rotation_arc(Vec3::Y, input.heading),
            sounds: vec![
                ("thmfrc_bi", 1.0),
                ("thmfrc_bc02", 1.0),
                ("thmfrc_ss", input.roll),
            ],
            ..SceneNodeSpec::default()
        },
        EffectKind::Bubble => SceneNodeSpec {
            material: Some(BARRIER_MATERIAL),
            gi: Some(GI_PROGRAM),
            block: BARRIER_BLOCK,
            position: input.body + Vec3::Y * 100.0,
            sounds: vec![
                ("thmfrc_bi", 1.0),
                ("thmfrc_bc02", 1.0),
                ("thmfrc_ss", input.roll),
                ("thmfrc_brf", 1.0),
            ],
            ..SceneNodeSpec::default()
        },
        EffectKind::Barricade => SceneNodeSpec {
            material: Some(BARRIER_MATERIAL),
            gi: Some(GI_PROGRAM),
            block: BARRIER_BLOCK,
            position: input.point.unwrap_or(input.body) + Vec3::Y * 110.0,
            rotation: Quat::from_rotation_arc(Vec3::Y, input.heading),
            sounds: vec![
                ("thmfrc_bi", 1.0),
                ("thmfrc_bc01", 1.0),
                ("thmfrc_ss", input.roll),
            ],
            ..SceneNodeSpec::default()
        },
        EffectKind::Shell => SceneNodeSpec {
            material: Some(AURA_MATERIAL),
            block: AURA_BLOCK,
            flags: EFFECT_NODE | SHELL_FLAG,
            position: input.body,
            sounds: vec![("thmfrc_ss", 0.6 + input.roll * 0.4)],
            ..SceneNodeSpec::default()
        },
        EffectKind::DecorativeShell => SceneNodeSpec {
            material: Some(AURA_MATERIAL),
            block: AURA_BLOCK,
            flags: 0x20,
            position: input.body,
            sounds: Vec::new(),
            ..SceneNodeSpec::default()
        },
        EffectKind::Vanish => SceneNodeSpec {
            position: input.body,
            sounds: vec![("thmdsp_apr", 1.0), ("thmdsp_vns", 1.0)],
            ..SceneNodeSpec::default()
        },
        EffectKind::LongDoor => SceneNodeSpec {
            position: input.point.unwrap_or(input.body),
            life_ms: 0,
            ..SceneNodeSpec::default()
        },
        EffectKind::Constant => SceneNodeSpec {
            material: Some(SHELL_MATERIAL),
            block: DISPLACEMENT_BLOCK,
            flags: EFFECT_NODE,
            position: input.body,
            sounds: vec![("thmdsp_cns01", input.roll), ("thmdsp_phs", 1.0)],
            ..SceneNodeSpec::default()
        },
        EffectKind::Summon => SceneNodeSpec {
            position: input.point.unwrap_or(input.body),
            life_ms: 0,
            ..SceneNodeSpec::default()
        },
        EffectKind::Tendril => SceneNodeSpec {
            material: Some(TENDRIL_MATERIAL),
            block: DISPLACEMENT_BLOCK,
            flags: EFFECT_NODE,
            position: input.body,
            ..SceneNodeSpec::default()
        },
        EffectKind::Kinesis => SceneNodeSpec {
            position: input.hand,
            sounds: vec![("thmfrc_knsst", 0.8 + input.roll * 0.4)],
            ..SceneNodeSpec::default()
        },
        EffectKind::Throw => SceneNodeSpec {
            position: input.hand,
            sounds: vec![
                ("thmfrc_knsst", 0.8 + input.roll * 0.4),
                ("thmfrc_throw", 0.8 + input.roll * 0.4),
            ],
            ..SceneNodeSpec::default()
        },
        EffectKind::Strikes => SceneNodeSpec {
            position: input.body,
            sounds: vec![("thmfrc_ss", 0.6 + input.roll * 0.4)],
            ..SceneNodeSpec::default()
        },
    }
}

/// The noise picture's bytes: four ridged fields of `NOISE_SIZE`, each a
/// channel. The original's noise object draws from its own generator; this
/// uses the engine's ridged float noise with a seed per channel.
pub fn noise_pixels(seed: u32) -> Vec<u8> {
    let (width, height, channels) = NOISE_SIZE;
    let mut out = Vec::with_capacity((width * height * channels) as usize);
    let mut planes = Vec::new();
    for channel in 0..channels {
        let plane = exa_render::image_noise::ridged_noise_floats(
            width as usize,
            height as usize,
            4,
            seed.wrapping_add(channel.wrapping_mul(0x9E37_79B9)),
        );
        planes.push(plane);
    }
    for index in 0..(width * height) as usize {
        for plane in &planes {
            let value = plane.get(index).copied().unwrap_or(0.0);
            out.push(((value + 0.5) * 255.0).clamp(0.0, 255.0) as u8);
        }
    }
    out
}

/// One node of a displacement shell as its render pass sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellNodeView {
    /// The node has subobjects (bit 0x1000_0000 of its render flags).
    pub subobjects: bool,
    /// The node's visible bit 0x8000.
    pub visible: bool,
    /// Its subobjects' render flags.
    pub parts: [u32; 4],
    pub part_count: usize,
}

/// A drawn shell node: its parts' count and whether it takes the thick
/// pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellDraw {
    pub thick: bool,
    pub parts: usize,
}

/// The shell pass's selection: a node with subobjects draws its parts whose
/// flags lack 0x80040; `visible` picks the thick pass (0x8000 set).
pub fn shell_draws(nodes: &[ShellNodeView], shown: f32) -> Vec<ShellDraw> {
    nodes
        .iter()
        .filter(|node| node.subobjects)
        .map(|node| {
            let parts = node.parts[..node.part_count.min(4)]
                .iter()
                .filter(|flags| **flags & 0x0008_0040 == 0)
                .count();
            let _ = shown;
            ShellDraw {
                thick: node.visible,
                parts,
            }
        })
        .collect()
}

/// The effect shader's per-layer texture offsets: the original walks the
/// effect's layers and sets a scrolling texture matrix before each draw.
/// A centred pass (the effect shader's own) steps by `0.002` a layer and
/// starts `0.001` back; the other starts `speed · 0.5` back and steps by
/// `speed`.
pub fn scroll_offsets(layers: i32, frame: i32, base: f32, speed: f32, centred: bool) -> Vec<f32> {
    if layers <= 0 {
        return Vec::new();
    }
    (0..layers)
        .map(|layer| {
            if centred {
                ((layer + frame) as f32 + 0.5) * 0.002 / layers as f32 + base - 0.001
            } else {
                ((layer + frame) as f32 + 0.5) * speed / layers as f32 + base - speed * 0.5
            }
        })
        .collect()
}

/// A fade hook: a callback and its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FadeHook {
    pub callback: u32,
    pub target: u64,
}

/// A body's fade-hook list; the original keeps up to 100.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FadeHooks {
    pub entries: Vec<FadeHook>,
}

impl FadeHooks {
    /// Adds a hook when the list has room.
    pub fn add(&mut self, callback: u32, target: u64, room: usize) -> bool {
        if self.entries.len() >= room {
            return false;
        }
        self.entries.push(FadeHook { callback, target });
        true
    }

    /// Removes and returns the hook at `index`.
    pub fn remove(&mut self, index: usize) -> Option<FadeHook> {
        (index < self.entries.len()).then(|| self.entries.remove(index))
    }

    /// The first hook with `callback`.
    pub fn find(&self, callback: u32) -> Option<usize> {
        self.entries
            .iter()
            .position(|hook| hook.callback == callback)
    }
}

/// A tendril part: its anchor in the body part's frame (cm) and the
/// texture it draws.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TendrilPart {
    pub anchor: [f32; 3],
}

impl TendrilPart {
    pub fn texture(&self) -> &'static str {
        TENDRIL_TEXTURE
    }
}

/// The tendril strands a set of body bones makes, from the same anchor
/// table the wisps use.
pub fn tendril_parts(bones: &[usize]) -> Vec<TendrilPart> {
    bones
        .iter()
        .flat_map(|&bone| {
            wisp_anchors(bone)
                .iter()
                .map(|anchor| TendrilPart { anchor: *anchor })
        })
        .collect()
}

/// Places a body at `to`, keeping its pose, and wakes it. `to` is the body
/// root's new place; the original's dynamics call also takes a height
/// offset that Exarust does not model (a guess). Returns the root's old
/// place.
pub fn teleport(doll: &mut Ragdoll, to: Vec3, orientation: Option<Quat>) -> Vec3 {
    let old = doll
        .parts
        .first()
        .map(|part| Vec3::from(part.transform().translation))
        .unwrap_or(Vec3::ZERO);
    let rotation = orientation.unwrap_or(Quat::IDENTITY);
    let delta =
        Affine3A::from_rotation_translation(rotation, to) * Affine3A::from_translation(-old);
    let worlds: Vec<Affine3A> = doll
        .parts
        .iter()
        .map(|part| delta * part.transform())
        .collect();
    doll.place_at(&worlds);
    doll.wake();
    old
}

/// A body's effect node in the scene.
#[derive(Component, Debug, Clone)]
pub struct EffectVisual {
    pub material: Option<&'static str>,
    pub gi: Option<&'static str>,
    pub flags: u32,
    pub block: usize,
    pub life_ms: u32,
}

/// A request to add an effect node to the scene.
#[derive(Message, Debug, Clone)]
pub struct VisualSpawn(pub SceneNodeSpec);

/// Adds [`EffectVisual`] entities for every [`VisualSpawn`] message.
pub struct VisualPlugin;

impl Plugin for VisualPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<VisualSpawn>()
            .add_systems(Update, spawn_visuals);
    }
}

fn spawn_visuals(mut commands: Commands, mut spawns: MessageReader<VisualSpawn>) {
    for VisualSpawn(spec) in spawns.read() {
        commands.spawn((
            Transform::from_translation(spec.position).with_rotation(spec.rotation),
            EffectVisual {
                material: spec.material,
                gi: spec.gi,
                flags: spec.flags,
                block: spec.block,
                life_ms: spec.life_ms,
            },
            Name::new(spec.material.unwrap_or("power node").to_string()),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_physics::body::{PartSetup, Ragdoll};

    fn input() -> CreatorInput {
        CreatorInput {
            body: Vec3::new(10.0, 0.0, 20.0),
            heading: Vec3::Z,
            hand: Vec3::new(12.0, 50.0, 18.0),
            point: Some(Vec3::new(100.0, 0.0, 50.0)),
            roll: 0.5,
        }
    }

    #[test]
    fn the_materials_bind_their_parameters() {
        assert_eq!(MATERIAL_PARAMS, [0.25, 0.25, 0.25, 1.0]);
        assert_eq!(FORCE_BLOCK, 0xf0);
        assert_eq!(BARRIER_BLOCK, 0xb8);
        assert_eq!(AURA_BLOCK, 0xf0);
        assert_eq!(DISPLACEMENT_BLOCK, 0xf0);
        assert_eq!(NOISE_SIZE, (256, 256, 4));
        assert_eq!(NOISE_LAYERS, 0x33);
        assert_eq!((NOISE_FLAGS, NOISE_USAGE), (0x0100_4200, 0x0200_7000));
        assert_eq!((GI_LIGHT, GI_ATTEN), ("Light", "Atten"));
    }

    #[test]
    fn a_blast_sits_in_the_hand_and_the_wave_on_the_body() {
        let blast = creator(EffectKind::Blast, &input());
        assert_eq!(blast.material, Some(FORCE_MATERIAL));
        assert_eq!(blast.flags, EFFECT_NODE);
        assert_eq!(blast.gi, Some(GI_PROGRAM));
        assert_eq!(blast.block, FORCE_BLOCK);
        assert_eq!(blast.position, input().hand);
        assert!(blast.hand);
        assert_eq!(blast.sounds, vec![("thmfrc_cu01", 1.0)]);
        let wave = creator(EffectKind::Wave, &input());
        assert_eq!(wave.position, input().body);
        assert!(!wave.hand);
        assert!(wave.sounds.iter().any(|sound| sound.0 == "thmfrc_wav"));
        assert_eq!(
            creator(EffectKind::Bolt, &input()).sounds[0].0,
            "thmfrc_blt"
        );
    }

    #[test]
    fn the_domes_stand_a_metre_up_and_the_barricade_on_its_point() {
        let barrier = creator(EffectKind::Barrier, &input());
        assert_eq!(barrier.material, Some(BARRIER_MATERIAL));
        assert_eq!(barrier.position, Vec3::new(10.0, 100.0, 20.0));
        assert_eq!(barrier.block, BARRIER_BLOCK);
        assert_eq!(barrier.sounds.len(), 3);
        let barricade = creator(EffectKind::Barricade, &input());
        assert_eq!(barricade.position, Vec3::new(100.0, 110.0, 50.0));
        assert_eq!(barricade.sounds[1].0, "thmfrc_bc01");
        let bubble = creator(EffectKind::Bubble, &input());
        assert_eq!(bubble.sounds[3].0, "thmfrc_brf");
    }

    #[test]
    fn the_shells_and_displacement_nodes_take_their_materials() {
        let shell = creator(EffectKind::Shell, &input());
        assert_eq!(shell.material, Some(AURA_MATERIAL));
        assert_eq!(shell.gi, None);
        assert_eq!(shell.flags, EFFECT_NODE | SHELL_FLAG);
        assert_eq!(shell.sounds, vec![("thmfrc_ss", 0.8)]);
        let decorative = creator(EffectKind::DecorativeShell, &input());
        assert_eq!(decorative.material, Some(AURA_MATERIAL));
        assert_eq!(decorative.flags, 0x20);
        assert!(decorative.sounds.is_empty());
        let constant = creator(EffectKind::Constant, &input());
        assert_eq!(constant.material, Some(SHELL_MATERIAL));
        assert_eq!(constant.sounds[0].0, "thmdsp_cns01");
        assert_eq!(constant.sounds[0].1, 0.5);
        assert_eq!(
            creator(EffectKind::Tendril, &input()).material,
            Some(TENDRIL_MATERIAL)
        );
    }

    #[test]
    fn the_effect_shader_scrolls_its_layers() {
        let centred = scroll_offsets(3, 10, 0.5, 1.0, true);
        assert_eq!(centred.len(), 3);
        assert_eq!(centred[0], ((10.0 + 0.5) * 0.002) / 3.0 + 0.5 - 0.001);
        let plain = scroll_offsets(2, 4, 0.25, 0.75, false);
        assert_eq!(plain[0], (4.5 * 0.75) / 2.0 + 0.25 - 0.375);
        assert!(scroll_offsets(0, 0, 0.0, 1.0, true).is_empty());
    }

    #[test]
    fn the_fade_hook_list_keeps_its_room_and_order() {
        let mut hooks = FadeHooks::default();
        assert!(hooks.add(7, 100, 2));
        assert!(hooks.add(9, 200, 2));
        assert!(!hooks.add(11, 300, 2));
        assert_eq!(hooks.find(9), Some(1));
        assert_eq!(
            hooks.remove(0),
            Some(FadeHook {
                callback: 7,
                target: 100
            })
        );
        assert_eq!(hooks.entries.len(), 1);
        assert_eq!(hooks.remove(4), None);
    }

    #[test]
    fn a_tendril_image_uses_the_wisp_anchors() {
        let parts = tendril_parts(&[1, 2, 8, 12]);
        // Bone 1 has four anchors, 2 has one, 8 none and 12 one.
        assert_eq!(parts.len(), 6);
        assert_eq!(parts[0].anchor, [-10.0, -10.0, 18.0]);
        assert_eq!(parts[0].texture(), TENDRIL_TEXTURE);
        assert_eq!(tendril_parts(&[]).len(), 0);
        assert_eq!(TENDRIL_SPARE, 8);
    }

    #[test]
    fn the_noise_picture_has_four_planes() {
        let pixels = noise_pixels(7);
        assert_eq!(pixels.len(), (256 * 256 * 4) as usize);
        assert_eq!(pixels, noise_pixels(7));
        assert_ne!(pixels, noise_pixels(8));
    }

    #[test]
    fn a_shell_pass_takes_the_visible_subobjects() {
        let nodes = [
            ShellNodeView {
                subobjects: true,
                visible: true,
                parts: [0, 0x40, 0x0008_0040, 0],
                part_count: 4,
            },
            ShellNodeView {
                subobjects: false,
                visible: true,
                parts: [0; 4],
                part_count: 0,
            },
            ShellNodeView {
                subobjects: true,
                visible: false,
                parts: [0, 0, 0, 0],
                part_count: 2,
            },
        ];
        let draws = shell_draws(&nodes, 0.5);
        assert_eq!(
            draws,
            vec![
                ShellDraw {
                    thick: true,
                    parts: 2
                },
                ShellDraw {
                    thick: false,
                    parts: 2
                }
            ]
        );
    }

    #[test]
    fn the_active_powers_carry_their_sounds() {
        assert_eq!(
            creator(EffectKind::Kinesis, &input()).sounds[0].0,
            "thmfrc_knsst"
        );
        assert_eq!(creator(EffectKind::Throw, &input()).sounds.len(), 2);
        assert_eq!(creator(EffectKind::Strikes, &input()).sounds[0].1, 0.8);
        assert_eq!(creator(EffectKind::Kinesis, &input()).material, None);
    }

    #[test]
    fn a_body_is_placed_at_the_new_point_and_wakes() {
        let setup = PartSetup {
            bone: 1,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::IDENTITY,
            centre: Vec3::ZERO,
            half: Vec3::splat(10.0),
            origin: Vec3::ZERO,
            links: Vec::new(),
            foot: None,
            pins: Vec::new(),
        };
        let mut doll = Ragdoll::new(&[setup]);
        let old = teleport(&mut doll, Vec3::new(100.0, 0.0, 50.0), None);
        assert_eq!(old, Vec3::ZERO);
        let centre = doll.parts[0]
            .positions
            .iter()
            .fold(Vec3::ZERO, |sum, p| sum + *p)
            / 4.0;
        assert!((centre - Vec3::new(100.0, 0.0, 50.0)).length() < 1e-3);
    }
}
