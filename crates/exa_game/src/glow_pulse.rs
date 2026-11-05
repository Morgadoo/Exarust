//! The game unit's glow pulse on a placed object, run on the live scene.
//!
//! A pulse is a task with the object, its start, the fade length and the
//! hold length ([`exa_script::objects::Objects::schedule_pulse`]). Every
//! frame each task writes its level, up over the fade, held, down over the
//! fade ([`exa_script::objects::pulse`]), into:
//!
//! - the object's own mesh when the object node is a mesh (type
//!   `& 0x5f == 9`): every part's emissive is rescaled by the level (the
//!   original sets the geometry's per-draw emissive bit `0x10` and its
//!   scale on every sub-object);
//! - each direct child node whose type `& 0x5f` is above 8: a mesh child's
//!   first part only, and a voxel light (type `0x41`) whose intensity
//!   becomes its record intensity times the level (the colour update
//!   follows, as the original's does).
//!
//! On the frame its time is up the task still writes a level of 0, then
//! ends. The level stays where the last frame left it: a pulse leaves its
//! object's glow and its lights dark. A level load cancels every task, as
//! the level reader clears the task list.
//!
//! Who pulses what: a scripted device's use (usable class 2, kind 3) runs
//! the object's script, plays the sound named in its usable section at 0.3
//! and pulses the object with fade 1200 and hold 0
//! ([`exa_script::objects::USE_PULSE`]). In the shipped data the one such
//! object is the base tier's "PedestalHand" (0x587, model `Pedestal Hand`,
//! one voxel light of intensity 40), placed in `exanima67`.
//!
//! The emissive rescale is Exarust's reading of the per-draw scale on its
//! material: each part gets its own material copy and its loaded emissive
//! is kept beside it ([`GlowBase`]). Node types are read from what the node
//! carries: a node with mesh parts is type 9 and a voxel light type 0x41.
//! [Exarust stand-in: the model reader keeps no node type word]
//!
//! Evidence: the object-use routine, the pulse task's creation and its
//! frame routine of the game unit (REA on `Exanima.exe`, checked against
//! the disassembly).

use bevy::prelude::*;
use exa_render::gi::VoxelLight;
use exa_render::material::ExaMaterial;
use exa_script::objects::{GlowChild, GlowWrite, Objects};
use exa_world::PartBounds;
use exa_world::node_parts::NodeParts;

/// The type word Exarust gives a node with mesh parts.
pub const MESH_NODE: u32 = 9;
/// The type word of a voxel light node.
pub const VOXEL_LIGHT_NODE: u32 = exa_render::gi::VOXEL_LIGHT_CLASS;

/// A pulse request: `object` (a placed object's entity) glows up over
/// `fade` ms, holds for `hold` ms and fades out again.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PulseGlow {
    pub object: Entity,
    pub fade: u32,
    pub hold: u32,
}

/// The running pulse tasks and the objects they drive. A task's object
/// number is its slot in [`Self::targets`].
#[derive(Resource, Debug, Default)]
pub struct GlowPulses {
    pub objects: Objects,
    pub targets: Vec<Entity>,
}

impl GlowPulses {
    /// The object number of `entity`, registering it once.
    fn number(&mut self, entity: Entity) -> u32 {
        match self.targets.iter().position(|&e| e == entity) {
            Some(at) => at as u32,
            None => {
                self.targets.push(entity);
                (self.targets.len() - 1) as u32
            }
        }
    }

    /// Queues a pulse at clock `now`.
    pub fn schedule(&mut self, object: Entity, fade: u32, hold: u32, now: u32) {
        let number = self.number(object);
        self.objects.schedule_pulse(number, fade, hold, now);
    }

    /// Cancels every task (a level load).
    pub fn cancel(&mut self) {
        self.objects.cancel_pulses();
        self.targets.clear();
    }
}

/// A part's emissive as it was loaded, kept once a pulse rescales it.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct GlowBase(pub Vec4);

/// The emissive of a part at a pulse `level`: the loaded colour times the
/// level, its fourth lane kept.
pub fn scaled_emissive(base: Vec4, level: f32) -> Vec4 {
    (base.truncate() * level).extend(base.w)
}

/// The type word of a node as the pulse reads it: a mesh with parts, a
/// voxel light, or neither.
pub fn node_kind(has_parts: bool, voxel_light: bool) -> u32 {
    if has_parts {
        MESH_NODE
    } else if voxel_light {
        VOXEL_LIGHT_NODE
    } else {
        0
    }
}

/// The scene the pulses read and write.
type NodeQuery<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static NodeParts>,
        Option<&'static Children>,
        Has<PartBounds>,
    ),
>;

/// The object node of a placed object: its first child that is a model
/// node (the original's object is the node itself; Exarust's placed entity
/// holds the model's root under it).
fn object_node(placed: Entity, nodes: &NodeQuery) -> Option<Entity> {
    let (_, children, _) = nodes.get(placed).ok()?;
    children?
        .iter()
        .find(|&child| nodes.get(child).is_ok_and(|(_, _, node)| node))
}

/// The direct child nodes of a node, in order.
fn child_nodes(node: Entity, nodes: &NodeQuery) -> Vec<Entity> {
    nodes
        .get(node)
        .ok()
        .and_then(|(_, children, _)| children)
        .map(|kids| {
            kids.iter()
                .filter(|&kid| nodes.get(kid).is_ok_and(|(_, _, node)| node))
                .collect()
        })
        .unwrap_or_default()
}

fn schedule_pulses(
    mut requests: MessageReader<PulseGlow>,
    mut pulses: ResMut<GlowPulses>,
    time: Option<Res<Time<Virtual>>>,
) {
    let now = time.map_or(0, |t| t.elapsed().as_millis() as u32);
    for request in requests.read() {
        pulses.schedule(request.object, request.fade, request.hold, now);
        info!(
            "glow pulse: {:?} fade {} hold {} from {now} ms",
            request.object, request.fade, request.hold
        );
    }
}

fn cancel_on_level(levels: Query<(), Added<exa_world::LevelRoot>>, mut pulses: ResMut<GlowPulses>) {
    if !levels.is_empty() && !pulses.objects.pulses.is_empty() {
        info!("glow pulse: a level load cancels every pulse");
        pulses.cancel();
    }
}

/// One frame of the pulse tasks: their writes into the parts' emissive
/// and the child lights.
#[allow(clippy::too_many_arguments)]
fn tick_pulses(
    mut commands: Commands,
    mut pulses: ResMut<GlowPulses>,
    time: Option<Res<Time<Virtual>>>,
    nodes: NodeQuery,
    mut lights: Query<&mut VoxelLight>,
    mut handles: Query<(&mut MeshMaterial3d<ExaMaterial>, Option<&GlowBase>)>,
    mut materials: ResMut<Assets<ExaMaterial>>,
    mut last_log: Local<u32>,
) {
    if pulses.objects.pulses.is_empty() {
        return;
    }
    let now = time.map_or(0, |t| t.elapsed().as_millis() as u32);
    let GlowPulses { objects, targets } = &mut *pulses;
    // The object number's node, its type word and its children.
    let resolve = |number: u32| -> Option<(Entity, Vec<Entity>)> {
        let placed = *targets.get(number as usize)?;
        let node = object_node(placed, &nodes)?;
        Some((node, child_nodes(node, &nodes)))
    };
    let lights_ro = &lights;
    let writes = objects.tick_pulses(now, |number| {
        let Some((node, children)) = resolve(number) else {
            return (0, Vec::new());
        };
        let has_parts = |e: Entity| {
            nodes
                .get(e)
                .is_ok_and(|(parts, _, _)| parts.is_some_and(|p| !p.parts.is_empty()))
        };
        let kind = node_kind(has_parts(node), lights_ro.contains(node));
        let children = children
            .iter()
            .map(|&child| {
                let light = lights_ro.get(child).ok();
                GlowChild::new(
                    node_kind(has_parts(child), light.is_some()),
                    light.map_or(0.0, |l| l.base_intensity),
                )
            })
            .collect();
        (kind, children)
    });
    let mut emissive = |part: Entity, level: f32, commands: &mut Commands| {
        let Ok((mut handle, base)) = handles.get_mut(part) else {
            return;
        };
        let base = match base {
            Some(base) => base.0,
            None => {
                // The part's own copy, so other meshes keep their glow.
                let Some(own) = materials.get(&handle.0).cloned() else {
                    return;
                };
                let base = own.params.emissive;
                handle.0 = materials.add(own);
                commands.entity(part).insert(GlowBase(base));
                base
            }
        };
        if let Some(mut material) = materials.get_mut(&handle.0) {
            material.params.emissive = scaled_emissive(base, level);
        }
    };
    let log = now.wrapping_sub(*last_log) >= 200;
    for write in &writes {
        match *write {
            GlowWrite::Mesh { object, level } => {
                let Some((node, _)) = resolve(object) else {
                    continue;
                };
                let parts = nodes
                    .get(node)
                    .ok()
                    .and_then(|(parts, _, _)| parts)
                    .map(|p| p.parts.clone())
                    .unwrap_or_default();
                for part in parts {
                    emissive(part, level, &mut commands);
                }
                if log {
                    info!("glow pulse: object {object} mesh level {level:.3}");
                }
            }
            GlowWrite::ChildMesh {
                object,
                child,
                level,
            } => {
                let Some((_, children)) = resolve(object) else {
                    continue;
                };
                let first = children.get(child).and_then(|&c| {
                    nodes
                        .get(c)
                        .ok()
                        .and_then(|(parts, _, _)| parts?.parts.first().copied())
                });
                if let Some(part) = first {
                    emissive(part, level, &mut commands);
                }
                if log {
                    info!("glow pulse: object {object} child mesh {child} level {level:.3}");
                }
            }
            GlowWrite::Light {
                object,
                child,
                intensity,
            } => {
                let Some((_, children)) = resolve(object) else {
                    continue;
                };
                if let Some(mut light) = children.get(child).and_then(|&c| lights.get_mut(c).ok()) {
                    light.set_intensity(intensity);
                }
                if log {
                    info!("glow pulse: object {object} light {child} intensity {intensity:.3}");
                }
            }
        }
    }
    if log {
        *last_log = now;
    }
    if objects.pulses.is_empty() {
        info!("glow pulse: the last pulse ended at {now} ms");
    }
}

/// The glow pulse tasks: requests, the level load's cancel and the frame
/// step.
pub struct GlowPulsePlugin;

impl Plugin for GlowPulsePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GlowPulses>()
            .add_message::<PulseGlow>()
            .add_systems(
                Update,
                (cancel_on_level, schedule_pulses, tick_pulses).chain(),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_emissive_follows_the_level_and_keeps_its_fourth_lane() {
        let base = Vec4::new(0.4, 0.8, 1.0, 0.5);
        assert_eq!(scaled_emissive(base, 0.5), Vec4::new(0.2, 0.4, 0.5, 0.5));
        assert_eq!(scaled_emissive(base, 0.0), Vec4::new(0.0, 0.0, 0.0, 0.5));
    }

    #[test]
    fn a_node_reads_as_a_mesh_a_light_or_neither() {
        assert_eq!(node_kind(true, false), 9);
        assert_eq!(node_kind(false, true), 0x41);
        assert_eq!(node_kind(false, false), 0);
        assert_eq!(exa_script::objects::channel(node_kind(true, true)), {
            Some(exa_script::objects::Channel::Emissive)
        });
    }

    fn bounds() -> PartBounds {
        PartBounds {
            centre: Vec3::ZERO,
            radius: 0.0,
            half_extents: Vec3::ZERO,
        }
    }

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Time<Virtual>>()
            .init_resource::<Assets<ExaMaterial>>()
            .add_plugins(GlowPulsePlugin);
        app
    }

    fn advance(app: &mut App, ms: u64) {
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .advance_by(std::time::Duration::from_millis(ms));
        app.update();
    }

    #[test]
    fn a_pulse_lights_the_child_light_and_leaves_it_dark() {
        let mut app = app();
        let placed = app.world_mut().spawn_empty().id();
        let node = app.world_mut().spawn((bounds(), ChildOf(placed))).id();
        let light = app
            .world_mut()
            .spawn((
                bounds(),
                VoxelLight::from_record([1.0, 0.89, 0.71], 40.0, 15.0),
                ChildOf(node),
            ))
            .id();
        app.world_mut().write_message(PulseGlow {
            object: placed,
            fade: 1200,
            hold: 0,
        });
        app.update();
        advance(&mut app, 600);
        let at = app.world().get::<VoxelLight>(light).unwrap().intensity;
        assert!((at - 20.0).abs() < 0.5, "{at}");
        advance(&mut app, 600);
        let at = app.world().get::<VoxelLight>(light).unwrap().intensity;
        assert!((at - 40.0).abs() < 0.5, "{at}");
        advance(&mut app, 1300);
        assert_eq!(app.world().get::<VoxelLight>(light).unwrap().intensity, 0.0);
        assert!(
            app.world()
                .resource::<GlowPulses>()
                .objects
                .pulses
                .is_empty()
        );
    }

    #[test]
    fn a_pulse_rescales_the_object_mesh_on_its_own_copy() {
        let mut app = app();
        let shared = app
            .world_mut()
            .resource_mut::<Assets<ExaMaterial>>()
            .add(ExaMaterial::default());
        app.world_mut()
            .resource_mut::<Assets<ExaMaterial>>()
            .get_mut(&shared)
            .unwrap()
            .params
            .emissive = Vec4::new(1.0, 0.5, 0.25, 1.0);
        let placed = app.world_mut().spawn_empty().id();
        let node = app.world_mut().spawn((bounds(), ChildOf(placed))).id();
        let part = app
            .world_mut()
            .spawn((MeshMaterial3d(shared.clone()), ChildOf(node)))
            .id();
        app.world_mut()
            .entity_mut(node)
            .insert(NodeParts::new(vec![part]));
        app.world_mut().write_message(PulseGlow {
            object: placed,
            fade: 1000,
            hold: 1000,
        });
        app.update();
        advance(&mut app, 1500);
        let own = app
            .world()
            .get::<MeshMaterial3d<ExaMaterial>>(part)
            .unwrap()
            .0
            .clone();
        assert_ne!(own, shared);
        let materials = app.world().resource::<Assets<ExaMaterial>>();
        assert_eq!(materials.get(&own).unwrap().params.emissive.x, 1.0);
        assert_eq!(
            materials.get(&shared).unwrap().params.emissive,
            Vec4::new(1.0, 0.5, 0.25, 1.0)
        );
    }
}
