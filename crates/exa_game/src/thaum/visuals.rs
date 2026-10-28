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
//! so it marks the node for the renderer rather than drawing it. The
//! materials, block sizes, GI programs, node flags, the barrier's raised
//! centre and the pitches named on the barricade, shell, strikes and
//! kinesis sounds follow the original; the other node positions, hand
//! attachments and orientations are the scene code's choices, not decoded.
//!
//! Evidence: the Force and Displacement creators, their materials and the
//! shared effect shader binding, the two per-pass texture-matrix callbacks,
//! the fade hook list, the decorative shell and the body placement query.

use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_formats::pixel::round_f32;
use exa_physics::body::Ragdoll;
use exa_render::image_noise::NoiseField;

use super::displacement::wisp_anchors;

use super::effect_material::{self, MaterialKind};
use super::force::{AURA_PARAM_SIZE, BARRIER_PARAM_SIZE, FORCE_PARAM_SIZE};
pub use super::force::{
    FORCE_AURA_MATERIAL as AURA_MATERIAL, FORCE_BARRIER_MATERIAL as BARRIER_MATERIAL,
    FORCE_MATERIAL, FORCE_NODE as EFFECT_NODE, GI_ATTEN, GI_LIGHT, GI_PROGRAM, MATERIAL_PARAMS,
    NOISE_SIZE, NOISE_TEXTURE, NOISE_WORD, NOISE_WORD_REDUCED,
};

/// The tendril's material.
pub const TENDRIL_MATERIAL: &str = "_TDispMat01";
/// The displacement shell's material.
pub const SHELL_MATERIAL: &str = "_EffectShell01";

/// The blast material's parameter block.
pub const FORCE_BLOCK: usize = FORCE_PARAM_SIZE;
/// The barrier material's parameter block.
pub const BARRIER_BLOCK: usize = BARRIER_PARAM_SIZE;
/// The aura material's parameter block.
pub const AURA_BLOCK: usize = AURA_PARAM_SIZE;
/// The tendril and shell materials' parameter block.
pub const DISPLACEMENT_BLOCK: usize = 0xf0;

/// The noise texture's sampler word.
pub const NOISE_FLAGS: u32 = 0x0100_4200;
/// The noise texture's usage word.
pub const NOISE_USAGE: u32 = 0x2007_0000;

/// The vertex attributes the effect shader binding declares.
pub const EFFECT_SHADER_ATTRIBUTES: u32 = 4;
/// A creator that returns no callback.
pub const NO_EFFECT_CALLBACK: u32 = 0;
/// The shell subobject's flag.
pub const SHELL_FLAG: u32 = 0x20;
/// The decorative shell's two extra fields.
pub const DECORATIVE_FLAG: u8 = 1;
pub const DECORATIVE_SCALE: f32 = 3.0;
/// The spare strands a tendril image keeps.
pub const TENDRIL_SPARE: usize = super::displacement::SPARE_STRANDS;
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
    /// The sounds the creator names, with their pitch. Most creators only
    /// look a sound up and store its id on the effect (loop sound, blast
    /// sound), which the pitch 1.0 stands for; the barricade's `thmfrc_ss` is
    /// played at its placement at a pitch of `0.8 + 0.4 · roll` (volume 0.3).
    pub sounds: Vec<(&'static str, f32)>,
    /// The sub-objects the material routine adds (one for the aura and the
    /// shell, none for the others).
    pub subobjects: usize,
    /// The textures the material binds.
    pub textures: Vec<&'static str>,
    /// The node's scale.
    pub scale: f32,
    /// The extra marker word a creator writes (the decorative shell's 1).
    pub marker: u32,
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
            subobjects: 0,
            textures: Vec::new(),
            scale: 1.0,
            marker: 0,
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
    /// The caster's chest (the shell's start sound sits there).
    pub chest: Vec3,
    /// The caster's body orientation; the domes take it as their node's.
    pub body_rotation: Quat,
    /// The horizontal direction the barricade faces away along.
    pub facing: Vec3,
    /// The effect's point, when it has one.
    pub point: Option<Vec3>,
    /// The random draw for a pitch (`0` to `1`).
    pub roll: f32,
}

/// Builds a creator's scene node.
///
/// Each kind's material is the routine the original's creator calls: the
/// force material for the blast family, the barrier material for the domes
/// and the barricade, the aura material for the shell (and its decorative
/// copy), the displacement material for the tendrils and the shell material
/// for a constant displacement. The aura and shell materials add one
/// sub-object; the barrier and aura materials bind the noise picture. The
/// node's orientation and place follow the creator's own writes: the domes
/// take the caster's body orientation, the barricade the direction away
/// from the caster, and the decorative shell is scaled three times.
pub fn creator(kind: EffectKind, input: &CreatorInput) -> SceneNodeSpec {
    let material = |kind: MaterialKind, position: Vec3, rotation: Quat| {
        let binding = kind.binding();
        SceneNodeSpec {
            material: Some(binding.name),
            flags: EFFECT_NODE | binding.node_flag,
            block: binding.block,
            position,
            rotation,
            subobjects: usize::from(binding.adds_subobject),
            textures: if binding.noise {
                vec![effect_material::NOISE_TEXTURE]
            } else {
                Vec::new()
            },
            ..SceneNodeSpec::default()
        }
    };
    let hand = |kind: MaterialKind, sound| SceneNodeSpec {
        gi: Some(GI_PROGRAM),
        hand: true,
        sounds: vec![sound],
        ..material(kind, input.hand, input.body_rotation)
    };
    match kind {
        EffectKind::Blast => hand(MaterialKind::Force, ("thmfrc_cu01", 1.0)),
        EffectKind::Bolt => SceneNodeSpec {
            sounds: vec![("thmfrc_blt", 1.0)],
            ..hand(MaterialKind::Force, ("thmfrc_cu01", 1.0))
        },
        EffectKind::Wave => SceneNodeSpec {
            position: input.body,
            hand: false,
            sounds: vec![("thmfrc_wav", 1.0), ("thmfrc_cu01", 1.0)],
            ..hand(MaterialKind::Force, ("thmfrc_cu01", 1.0))
        },
        EffectKind::Torrent => SceneNodeSpec {
            sounds: vec![("thmfrc_cu01", 1.0), ("thmfrc_trntdr", 1.0)],
            ..hand(MaterialKind::Force, ("thmfrc_cu01", 1.0))
        },
        EffectKind::Barrier => SceneNodeSpec {
            gi: Some(GI_PROGRAM),
            sounds: vec![("thmfrc_bi", 1.0), ("thmfrc_bc02", 1.0)],
            ..material(
                MaterialKind::Barrier,
                input.body + Vec3::Y * 100.0,
                input.body_rotation,
            )
        },
        EffectKind::Bubble => SceneNodeSpec {
            gi: Some(GI_PROGRAM),
            sounds: vec![
                ("thmfrc_bi", 1.0),
                ("thmfrc_bc02", 1.0),
                ("thmfrc_brf", 1.0),
            ],
            ..material(
                MaterialKind::Barrier,
                input.body + Vec3::Y * 100.0,
                input.body_rotation,
            )
        },
        EffectKind::Barricade => SceneNodeSpec {
            gi: Some(GI_PROGRAM),
            position: input.point.unwrap_or(input.body) + Vec3::Y * 110.0,
            rotation: Quat::from_rotation_arc(Vec3::Y, input.facing),
            sounds: vec![
                ("thmfrc_bi", 1.0),
                ("thmfrc_bc01", 1.0),
                ("thmfrc_ss", 0.8 + input.roll * 0.4),
            ],
            ..material(
                MaterialKind::Barrier,
                input.point.unwrap_or(input.body) + Vec3::Y * 110.0,
                Quat::from_rotation_arc(Vec3::Y, input.facing),
            )
        },
        EffectKind::Shell => SceneNodeSpec {
            flags: EFFECT_NODE | effect_material::AURA_FLAGS | effect_material::SHELL_FLAG,
            position: input.body,
            sounds: vec![("thmfrc_ss", 0.6 + input.roll * 0.4)],
            ..material(MaterialKind::Aura, input.body, Quat::IDENTITY)
        },
        EffectKind::DecorativeShell => SceneNodeSpec {
            flags: EFFECT_NODE | effect_material::AURA_FLAGS | effect_material::SHELL_FLAG,
            scale: DECORATIVE_SCALE,
            marker: 1,
            position: input.body,
            sounds: Vec::new(),
            ..material(MaterialKind::Aura, input.body, Quat::IDENTITY)
        },
        EffectKind::Vanish => SceneNodeSpec {
            position: input.body,
            sounds: vec![("thmdsp_apr", 1.0), ("thmdsp_vns", 1.0)],
            ..SceneNodeSpec::default()
        },
        EffectKind::LongDoor => SceneNodeSpec {
            position: input.point.unwrap_or(input.body),
            ..SceneNodeSpec::default()
        },
        EffectKind::Constant => SceneNodeSpec {
            flags: EFFECT_NODE | effect_material::AURA_FLAGS | effect_material::SHELL_FLAG,
            position: input.body,
            sounds: vec![("thmdsp_cns01", input.roll), ("thmdsp_phs", 1.0)],
            ..material(MaterialKind::Shell, input.body, Quat::IDENTITY)
        },
        EffectKind::Summon => SceneNodeSpec {
            position: input.point.unwrap_or(input.body),
            ..SceneNodeSpec::default()
        },
        EffectKind::Tendril => SceneNodeSpec {
            flags: EFFECT_NODE | effect_material::MATERIAL_FLAG,
            position: input.body,
            ..material(MaterialKind::Tendril, input.body, Quat::IDENTITY)
        },
        EffectKind::Kinesis => SceneNodeSpec {
            position: input.hand,
            sounds: vec![("thmfrc_knsst", 0.8 + input.roll * 0.4)],
            ..SceneNodeSpec::default()
        },
        EffectKind::Throw => SceneNodeSpec {
            position: input.chest,
            sounds: vec![
                ("thmfrc_knsst", 0.8 + input.roll * 0.4),
                ("thmfrc_throw", 0.8 + input.roll * 0.4),
            ],
            ..SceneNodeSpec::default()
        },
        EffectKind::Strikes => SceneNodeSpec {
            position: input.chest,
            sounds: vec![("thmfrc_ss", 0.6 + input.roll * 0.4)],
            ..SceneNodeSpec::default()
        },
    }
}

/// The animated noise field the Force effects share: 256 by 256 at four
/// octaves and amplitude 1, both mixing weights 0.5, then set to four
/// octaves, a 0.8 s phase and a target amplitude of 1. `seeds` supplies the
/// engine's random words (four ridged planes, then the first white noise)
/// and `now` is the clock in milliseconds.
pub fn noise_field(now: u32, seeds: impl FnMut() -> u32) -> NoiseField {
    let (width, height, octaves) = NOISE_SIZE;
    let mut field = NoiseField::new(
        width as usize,
        height as usize,
        octaves as i32,
        1.0,
        now,
        false,
        seeds,
    );
    field.keep = 0.5;
    field.add = 0.5;
    field.configure(4, 0.8, 1.0);
    field
}

/// The picture the field uploads to its one-channel texture: each float
/// from 0 to 1 scaled by 255 and rounded to even; at or below 0 is 0 and at
/// or above 1 is 255 (a not-a-number scales to the indefinite integer, whose
/// low byte is 0).
pub fn noise_bytes(field: &NoiseField) -> Vec<u8> {
    float_bytes(field.output())
}

/// The float-to-byte conversion of [`noise_bytes`].
pub fn float_bytes(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .map(|&v| {
            if v <= 0.0 {
                0
            } else if v >= 1.0 {
                255
            } else {
                // A not-a-number lands here and scales to the indefinite
                // integer, whose low byte is 0.
                round_f32(v * 255.0) as u8
            }
        })
        .collect()
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
/// The shader's own pass is centred: in double precision it takes the layer
/// plus the frame counter (summed as a single, half added), scales by `0.002`,
/// divides by a global divisor that stays at 1 (not the layer count), adds
/// the base and takes `0.001` back, then narrows once. The other pass
/// works in single precision from the effect's own `speed` and starts half a
/// `speed` back.
pub fn scroll_offsets(layers: i32, frame: i32, base: f32, speed: f32, centred: bool) -> Vec<f32> {
    if layers <= 0 {
        return Vec::new();
    }
    (0..layers)
        .map(|layer| {
            let slot = ((i64::from(layer) + i64::from(frame)) as f32) + 0.5;
            if centred {
                (f64::from(slot) * 0.002 / SCROLL_DIVISOR + f64::from(base) - 0.001) as f32
            } else {
                slot * speed / SCROLL_DIVISOR as f32 + base - speed * 0.5
            }
        })
        .collect()
}

/// The scroll divisor the original keeps in a global; it holds 1.
const SCROLL_DIVISOR: f64 = 1.0;

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
    /// The sub-objects the material added.
    pub subobjects: usize,
    /// The textures the material bound.
    pub textures: Vec<&'static str>,
    /// The node's scale.
    pub scale: f32,
    /// The creator's marker word.
    pub marker: u32,
}

impl EffectVisual {
    /// The material routine this node's material came from, when it has one.
    pub fn material_kind(&self) -> Option<MaterialKind> {
        let name = self.material?;
        [
            MaterialKind::Force,
            MaterialKind::Barrier,
            MaterialKind::Aura,
            MaterialKind::Tendril,
            MaterialKind::Shell,
        ]
        .into_iter()
        .find(|kind| kind.name() == name)
    }

    /// The draws this node's material pass makes, as its material runs
    /// them. A node without a material draws nothing.
    pub fn pass_plan(
        &self,
        params: &super::effect_pass::PassParams,
    ) -> Vec<super::effect_pass::PassDraw> {
        let Some(kind) = self.material_kind() else {
            return Vec::new();
        };
        super::effect_pass::plan(kind.draw(), params)
    }
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

/// Adds one effect node to the scene and returns it.
pub fn spawn_visual(commands: &mut Commands, spec: SceneNodeSpec) -> Entity {
    commands
        .spawn((
            Transform::from_translation(spec.position)
                .with_rotation(spec.rotation)
                .with_scale(Vec3::splat(spec.scale)),
            EffectVisual {
                material: spec.material,
                gi: spec.gi,
                flags: spec.flags,
                block: spec.block,
                life_ms: spec.life_ms,
                subobjects: spec.subobjects,
                textures: spec.textures.clone(),
                scale: spec.scale,
                marker: spec.marker,
            },
            Name::new(spec.material.unwrap_or("power node").to_string()),
        ))
        .id()
}

fn spawn_visuals(mut commands: Commands, mut spawns: MessageReader<VisualSpawn>) {
    for VisualSpawn(spec) in spawns.read() {
        spawn_visual(&mut commands, spec.clone());
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
            chest: Vec3::new(10.0, 120.0, 20.0),
            body_rotation: Quat::IDENTITY,
            facing: Vec3::Z,
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
        assert_eq!((NOISE_WORD, NOISE_WORD_REDUCED), (0x2f0, 0x33));
        assert_eq!((NOISE_FLAGS, NOISE_USAGE), (0x0100_4200, 0x2007_0000));
        assert_eq!((GI_LIGHT, GI_ATTEN), ("Light", "Atten"));
    }

    #[test]
    fn a_blast_sits_in_the_hand_and_the_wave_on_the_body() {
        let blast = creator(EffectKind::Blast, &input());
        assert_eq!(blast.material, Some(FORCE_MATERIAL));
        assert_eq!(blast.flags, EFFECT_NODE | effect_material::MATERIAL_FLAG);
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
        assert_eq!(barrier.sounds.len(), 2);
        let barricade = creator(EffectKind::Barricade, &input());
        assert_eq!(barricade.position, Vec3::new(100.0, 110.0, 50.0));
        assert_eq!(barricade.sounds[1].0, "thmfrc_bc01");
        assert_eq!(barricade.sounds[2], ("thmfrc_ss", 0.8 + 0.5 * 0.4));
        let bubble = creator(EffectKind::Bubble, &input());
        assert_eq!(bubble.sounds[2].0, "thmfrc_brf");
    }

    #[test]
    fn the_shells_and_displacement_nodes_take_their_materials() {
        let shell = creator(EffectKind::Shell, &input());
        assert_eq!(shell.material, Some(AURA_MATERIAL));
        assert_eq!(shell.gi, None);
        assert_eq!(
            shell.flags,
            EFFECT_NODE | effect_material::AURA_FLAGS | SHELL_FLAG
        );
        assert_eq!(shell.sounds, vec![("thmfrc_ss", 0.8)]);
        assert_eq!(shell.subobjects, 1);
        assert_eq!(shell.textures, vec![effect_material::NOISE_TEXTURE]);
        let decorative = creator(EffectKind::DecorativeShell, &input());
        assert_eq!(decorative.material, Some(AURA_MATERIAL));
        assert_eq!(
            decorative.flags,
            EFFECT_NODE | effect_material::AURA_FLAGS | SHELL_FLAG
        );
        assert_eq!((decorative.scale, decorative.marker), (3.0, 1));
        assert!(decorative.sounds.is_empty());
        let constant = creator(EffectKind::Constant, &input());
        assert_eq!(constant.material, Some(SHELL_MATERIAL));
        assert_eq!(constant.subobjects, 1);
        assert!(constant.textures.is_empty());
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
        assert_eq!(centred[0], ((10.5f64 * 0.002) + 0.5 - 0.001) as f32);
        assert_eq!(centred[2], ((12.5f64 * 0.002) + 0.5 - 0.001) as f32);
        let plain = scroll_offsets(2, 4, 0.25, 0.75, false);
        assert_eq!(plain[0], 4.5f32 * 0.75 + 0.25 - 0.375);
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
    fn the_noise_picture_is_one_channel() {
        let mut state = 7u32;
        let mut seeds = || {
            state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
            state
        };
        let field = noise_field(0, &mut seeds);
        assert_eq!((field.keep, field.add), (0.5, 0.5));
        let pixels = noise_bytes(&field);
        assert_eq!(pixels.len(), 256 * 256);
    }

    #[test]
    fn the_bytes_clamp_and_round_to_even() {
        assert_eq!(
            float_bytes(&[-1.0, 0.0, 1.0, 2.0, f32::NAN]),
            [0, 0, 255, 255, 0]
        );
        // 0.5 * 255 = 127.5 rounds to the even 128; 2^-7 * 255 is 1.99 and
        // 2^-9 * 255 is 0.498.
        assert_eq!(float_bytes(&[0.5, 0.007_812_5, 0.001_953_125]), [128, 2, 0]);
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
            spheres: None,
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
    #[test]
    fn each_node_takes_its_material_and_its_pass_plan() {
        let shell = creator(EffectKind::Shell, &input());
        let visual = EffectVisual {
            material: shell.material,
            gi: shell.gi,
            flags: shell.flags,
            block: shell.block,
            life_ms: shell.life_ms,
            subobjects: shell.subobjects,
            textures: shell.textures,
            scale: shell.scale,
            marker: shell.marker,
        };
        assert_eq!(visual.material_kind(), Some(MaterialKind::Aura));
        let params = super::super::effect_pass::PassParams {
            passes: 2,
            frame: 0,
            base: 0.0,
            speed: 0.5,
            ..super::super::effect_pass::PassParams::default()
        };
        let plan = visual.pass_plan(&params);
        assert_eq!(plan.len(), 3);
        // The aura's shader pass rows use the node's speed.
        assert_eq!(plan[1].row, ((1.5 * 0.5) - 0.25));
        let blast = creator(EffectKind::Blast, &input());
        let visual = EffectVisual {
            material: blast.material,
            gi: blast.gi,
            flags: blast.flags,
            block: blast.block,
            life_ms: blast.life_ms,
            subobjects: blast.subobjects,
            textures: blast.textures,
            scale: blast.scale,
            marker: blast.marker,
        };
        assert_eq!(visual.material_kind(), Some(MaterialKind::Force));
        // The force material's rows step by 0.002.
        let plan = visual.pass_plan(&params);
        assert!((plan[1].row - (1.5 * 0.002 - 0.001)).abs() < 1e-6);
        assert_eq!(creator(EffectKind::Vanish, &input()).material, None);
    }
}
