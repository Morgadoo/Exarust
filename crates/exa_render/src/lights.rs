//! What the surface shaders know about lights beyond Bevy's clustered data:
//! a record per light (its vertical shaping) and, per mesh, the lights it is
//! lit by. The original lights each object with at most its six strongest
//! linked lights, not every light in reach.
//!
//! Each light gets an id, carried to the shaders in its `shadow_depth_bias`
//! field (Exarust's own shadow filter does not use Bevy's biases). A mesh
//! with a light list gets a slot, carried in its mesh tag above the
//! shadow-bias level (bits 0–2): tag = level | (slot + 1) << 3. Slot 0 means
//! no list: every light in reach. Rules: `docs/RENDERING.md` (Lights).
//!
//! Records and lists share one storage buffer, bound to every surface
//! material at binding 130. The buffer is stored under the default asset id,
//! so a material's default handle already points at it.

use bevy::asset::RenderAssetUsages;
use bevy::ecs::entity::EntityHashMap;
use bevy::mesh::MeshTag;
use bevy::prelude::*;
use bevy::render::storage::ShaderBuffer;

pub struct LightsPlugin;

impl Plugin for LightsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LightTable>()
            .configure_sets(
                PostUpdate,
                (
                    LightsSystems::Ids,
                    LightsSystems::Cubes,
                    LightsSystems::Lists,
                )
                    .chain(),
            )
            .add_systems(Startup, create_buffer)
            .add_systems(PostUpdate, assign_light_ids.in_set(LightsSystems::Ids))
            .add_systems(
                PostUpdate,
                (cube_records, assign_mesh_slots, free_mesh_slots, upload)
                    .chain()
                    .in_set(LightsSystems::Lists),
            );
    }
}

/// Light ids are given out in `Ids`; cached cube slots are handed out in
/// `Cubes`; mesh light lists and records are written to the shaders' buffer
/// in `Lists`. Systems that fill [`MeshLights`] run before `Cubes`.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LightsSystems {
    Ids,
    Cubes,
    Lists,
}

/// The most lights a mesh is lit by, as in the original.
pub const MAX_MESH_LIGHTS: usize = 6;

/// A light's vertical shaping (y, z) for the shaders; lights without it are
/// unshaped (1, 1).
#[derive(Component, Clone, Copy, Debug)]
pub struct LightShaping(pub Vec2);

/// A light's id in the shaders' records.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LightId(pub u32);

/// The lights a mesh is lit by (light ids, strongest first, at most
/// [`MAX_MESH_LIGHTS`]). Meshes without it are lit by every light in reach.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct MeshLights(pub Vec<u32>);

/// A mesh's shadow-bias level (see [`crate::shadow_bias`]).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShadowBias(pub u32);

/// A light's record: shaping y, z, cached cube slot + 1 (0 for none), and
/// its radius R in metres.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Record {
    shaping: Vec2,
    cube: u32,
    radius: f32,
}

impl Default for Record {
    fn default() -> Self {
        Self {
            shaping: Vec2::ONE,
            cube: 0,
            radius: 1.0,
        }
    }
}

#[derive(Resource, Default)]
struct LightTable {
    records: Vec<Record>,
    /// Per mesh slot: count, then the light ids.
    lists: Vec<[u32; 8]>,
    free: Vec<u32>,
    /// Which mesh holds each slot.
    slot_of: EntityHashMap<u32>,
    dirty: bool,
}

/// The buffer's layout, as `array<vec4<u32>>`: element 0 holds the index of
/// the first list; element 1 + id holds light `id`'s record (shaping y, z and
/// radius as f32 bits, cube slot + 1); each mesh slot takes two elements
/// from the first list on.
fn layout(table: &LightTable) -> Vec<[u32; 4]> {
    let first_list = 1 + table.records.len().max(1) as u32;
    let mut data = Vec::with_capacity(first_list as usize + table.lists.len() * 2 + 2);
    data.push([first_list, 0, 0, 0]);
    data.extend(table.records.iter().map(|r| {
        [
            r.shaping.x.to_bits(),
            r.shaping.y.to_bits(),
            r.cube,
            r.radius.to_bits(),
        ]
    }));
    if table.records.is_empty() {
        data.push([1f32.to_bits(), 1f32.to_bits(), 0, 1f32.to_bits()]);
    }
    for list in &table.lists {
        data.push([list[0], list[1], list[2], list[3]]);
        data.push([list[4], list[5], list[6], list[7]]);
    }
    // Never empty: a storage binding needs at least one element per read.
    data.push([0; 4]);
    data
}

fn create_buffer(mut buffers: ResMut<Assets<ShaderBuffer>>) {
    let data = layout(&LightTable::default());
    let _ = buffers.insert(
        AssetId::default(),
        ShaderBuffer::new(data, RenderAssetUsages::RENDER_WORLD),
    );
}

/// Gives every new light an id, carried in its depth-bias field.
fn assign_light_ids(
    mut commands: Commands,
    mut table: ResMut<LightTable>,
    mut lights: Query<(Entity, &mut PointLight, Option<&LightShaping>), Without<LightId>>,
    mut shaped: Query<(&LightId, &LightShaping), Changed<LightShaping>>,
) {
    for (entity, mut light, shaping) in &mut lights {
        let id = table.records.len() as u32;
        table.records.push(Record {
            shaping: shaping.map_or(Vec2::ONE, |s| s.0),
            ..default()
        });
        light.shadow_depth_bias = id as f32;
        commands.entity(entity).insert(LightId(id));
        table.dirty = true;
    }
    for (id, shaping) in &mut shaped {
        if let Some(r) = table.records.get_mut(id.0 as usize)
            && r.shaping != shaping.0
        {
            r.shaping = shaping.0;
            table.dirty = true;
        }
    }
}

/// Writes lights' cached cube slots into their records.
fn cube_records(
    mut table: ResMut<LightTable>,
    cubes: Query<
        (&LightId, &crate::cube_shadows::CubeSlot),
        Changed<crate::cube_shadows::CubeSlot>,
    >,
) {
    for (id, cube) in &cubes {
        let Some(r) = table.records.get_mut(id.0 as usize) else {
            continue;
        };
        let new = Record {
            cube: cube.slot.map_or(0, |s| s + 1),
            radius: cube.radius,
            ..*r
        };
        if *r != new {
            *r = new;
            table.dirty = true;
        }
    }
}

/// The tag of a mesh with bias level `bias` and list slot `slot`.
fn tag(bias: u32, slot: Option<u32>) -> u32 {
    (bias & 7) | slot.map_or(0, |s| (s + 1) << 3)
}

type SlotQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        Option<&'static MeshLights>,
        Option<&'static ShadowBias>,
    ),
    Or<(Changed<MeshLights>, Changed<ShadowBias>)>,
>;

/// Writes the light lists of meshes whose lists changed and gives them
/// their tags.
fn assign_mesh_slots(mut commands: Commands, mut table: ResMut<LightTable>, meshes: SlotQuery) {
    for (entity, lights, bias) in &meshes {
        let slot = lights.map(|lights| {
            let s = match table.slot_of.get(&entity) {
                Some(&s) => s,
                None => {
                    let s = table.free.pop().unwrap_or_else(|| {
                        table.lists.push([0; 8]);
                        table.lists.len() as u32 - 1
                    });
                    table.slot_of.insert(entity, s);
                    s
                }
            };
            let mut list = [0u32; 8];
            let n = lights.0.len().min(MAX_MESH_LIGHTS);
            list[0] = n as u32;
            list[1..1 + n].copy_from_slice(&lights.0[..n]);
            if table.lists[s as usize] != list {
                table.lists[s as usize] = list;
                table.dirty = true;
            }
            s
        });
        let value = tag(bias.map_or(0, |b| b.0), slot);
        commands.entity(entity).insert(MeshTag::new(value));
    }
}

/// Returns the slots of meshes that lost their list or were despawned.
fn free_mesh_slots(
    mut commands: Commands,
    mut table: ResMut<LightTable>,
    mut removed: RemovedComponents<MeshLights>,
    biases: Query<Option<&ShadowBias>, Without<MeshLights>>,
) {
    for entity in removed.read() {
        let Some(s) = table.slot_of.remove(&entity) else {
            continue;
        };
        table.lists[s as usize] = [0; 8];
        table.free.push(s);
        table.dirty = true;
        // A mesh still alive keeps its bias level, lit by every light.
        if let Ok(bias) = biases.get(entity) {
            commands
                .entity(entity)
                .insert(MeshTag::new(tag(bias.map_or(0, |b| b.0), None)));
        }
    }
}

fn upload(mut table: ResMut<LightTable>, mut buffers: ResMut<Assets<ShaderBuffer>>) {
    if !table.dirty {
        return;
    }
    table.dirty = false;
    let data = layout(&table);
    let _ = buffers.insert(
        AssetId::default(),
        ShaderBuffer::new(data, RenderAssetUsages::RENDER_WORLD),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_pack_bias_and_slot() {
        assert_eq!(tag(3, None), 3);
        assert_eq!(tag(1, Some(0)), 1 | 8);
        assert_eq!(tag(9, Some(4)), 1 | 40);
    }

    #[test]
    fn layout_places_records_then_lists() {
        let table = LightTable {
            records: vec![
                Record {
                    shaping: Vec2::new(0.5, 2.0),
                    cube: 3,
                    radius: 7.5,
                },
                Record::default(),
            ],
            lists: vec![[2, 7, 9, 0, 0, 0, 0, 0]],
            ..default()
        };
        let data = layout(&table);
        assert_eq!(data[0][0], 3);
        assert_eq!(f32::from_bits(data[1][0]), 0.5);
        assert_eq!(f32::from_bits(data[1][1]), 2.0);
        assert_eq!(data[1][2], 3);
        assert_eq!(f32::from_bits(data[1][3]), 7.5);
        assert_eq!(data[3], [2, 7, 9, 0]);
        assert_eq!(data[4], [0; 4]);
    }
}
