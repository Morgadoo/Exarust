//! What the surface shaders know about lights beyond Bevy's clustered data:
//! a record per light (its vertical shaping) and, per mesh, the lights it is
//! lit by. The original lights each object with at most its six strongest
//! linked lights, not every light in reach.
//!
//! Each light gets an id, carried to the shaders in its `shadow_depth_bias`
//! field above Bevy's default point-light bias. The field must stay a valid
//! bias: a skinned character's shadow is rendered by Bevy's own map, and an
//! integer id (up to a few hundred) would push its shadow away from the
//! caster. Exarust's own shadow filter reads only the fractional id above the
//! default. A mesh with a light list gets a slot, carried in its mesh tag
//! above the shadow-bias level (bits 0–2): tag = level | (slot + 1) << 3,
//! the slot within bits 3–19 (the GI cell, [`crate::gi_tag`], sits above).
//! Slot 0 means no list: every light in reach. Rules: `docs/RENDERING.md`
//! (Lights).
//!
//! Records and lists share one storage buffer, bound to every surface
//! material at binding 130. The buffer is stored under the default asset id,
//! so a material's default handle already points at it. Its first element
//! also carries the sun map's filter radius and caster polygon offset
//! ([`crate::sun_fit::SunShadowFilter`]), which the surface shaders and the
//! sun caster pass read from the same binding.

use bevy::asset::RenderAssetUsages;
use bevy::ecs::entity::EntityHashMap;
use bevy::mesh::MeshTag;
use bevy::prelude::*;
use bevy::render::storage::ShaderBuffer;

use crate::gi_tag::GiCellTag;

pub struct LightsPlugin;

impl Plugin for LightsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LightTable>()
            .init_resource::<crate::sun_fit::SunShadowFilter>()
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

/// Bevy's default point-light depth bias. Every level light keeps it, so the
/// skinned character shadows that still render through Bevy's maps retain the
/// same bias they had before Exarust gave lights ids.
pub const LIGHT_ID_BIAS: f32 = PointLight::DEFAULT_SHADOW_DEPTH_BIAS;

/// The depth-bias distance between successive light ids. The shader recovers
/// `round((bias - LIGHT_ID_BIAS) / LIGHT_ID_STEP)`; one step is far above
/// `f32`'s precision at the default bias, and a whole level's ids add only a
/// few thousandths to it.
pub const LIGHT_ID_STEP: f32 = 0.00001;

/// The depth-bias field carrying light `id` without disturbing Bevy's
/// shadow maps.
pub fn light_bias(id: u32) -> f32 {
    LIGHT_ID_BIAS + id as f32 * LIGHT_ID_STEP
}

/// The id carried by a light's depth-bias field.
pub fn light_id_of(bias: f32) -> u32 {
    ((bias - LIGHT_ID_BIAS) / LIGHT_ID_STEP).round().max(0.0) as u32
}

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

/// A drawable node's two shadow-depth scales from its render flags:
/// `(receive, cast)`.
///
/// - With bit `0x80000`, both are `1 + v·0.005` for the level `v` in bits
///   16–18, in double precision.
/// - Otherwise, with a level, receive is `1 − v·0.005` (double precision)
///   and cast is that less 0.01 (the original subtracts in double from the
///   single-precision receive; the result is the same at every level).
/// - Without either, both are 1.
///
/// The surface shader already applies the level-only rule
/// (`shadow_depth_scale`); this is the whole rule, kept for the
/// `0x80000` branch the shader lacks.
pub fn shadow_bias_scales(flags: u32) -> (f32, f32) {
    if flags & 0x8_0000 != 0 {
        let v = f64::from((flags & 0x7_0000) >> 16);
        let scale = (v * 0.005 + 1.0) as f32;
        return (scale, scale);
    }
    if flags & 0x7_0000 != 0 {
        let v = f64::from((flags & 0x7_0000) >> 16);
        let receive = (1.0 - v * 0.005) as f32;
        return (receive, receive - 0.01);
    }
    (1.0, 1.0)
}

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
    /// The sun map's filter and caster offset, in element 0.
    sun: crate::sun_fit::SunShadowFilter,
    /// The shadow taps one sub-frame takes (`SdwSmps`), in element 0's top
    /// byte; 0 until the first upload (the shaders then take 8).
    shadow_samples: u32,
    dirty: bool,
}

/// The buffer's layout, as `array<vec4<u32>>`: element 0 holds the index of
/// the first list (bits 0–23) with the sub-frame's shadow taps `SdwSmps`
/// above it ([`HEADER_TAPS_SHIFT`]), then the sun's filter radius, polygon
/// factor and polygon units as f32 bits; element 1 + id holds light `id`'s record (shaping y, z and
/// radius as f32 bits, cube slot + 1); each mesh slot takes two elements
/// from the first list on.
fn layout(table: &LightTable) -> Vec<[u32; 4]> {
    let first_list = 1 + table.records.len().max(1) as u32;
    let mut data = Vec::with_capacity(first_list as usize + table.lists.len() * 2 + 2);
    data.push([
        first_list | table.shadow_samples.min(0xFF) << HEADER_TAPS_SHIFT,
        table.sun.radius.to_bits(),
        table.sun.polygon_factor.to_bits(),
        table.sun.polygon_units.to_bits(),
    ]);
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

/// Where element 0 keeps `SdwSmps`: its top byte (the first list's index
/// stays below 2^24).
pub const HEADER_TAPS_SHIFT: u32 = 24;

/// The shadow taps one sub-frame takes for the settings screen's shadow
/// quality and the scene's super-sampling: the original's `SdwSmps`
/// ([`crate::shader_program::shadow_samples`]) at its 4× setting (3) when
/// the scene is supersampled, else at setting 0. 2 at the shipped settings
/// (shadow quality 4, supersampled), 8 without super-sampling.
pub fn sub_frame_taps(shadow_quality: i32, split: bool) -> u32 {
    crate::shader_program::shadow_samples(shadow_quality.max(0) as u32, if split { 3 } else { 0 })
}

fn create_buffer(mut buffers: ResMut<Assets<ShaderBuffer>>) {
    let data = layout(&LightTable::default());
    let _ = buffers.insert(
        AssetId::default(),
        ShaderBuffer::new(data, RenderAssetUsages::RENDER_WORLD),
    );
}

/// Gives every new light an id, carried above the default bias in its
/// depth-bias field.
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
        light.shadow_depth_bias = light_bias(id);
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

/// The tag of a mesh with bias level `bias`, list slot `slot` and GI cell
/// bits `gi` ([`crate::gi_tag`], bits 20 and up).
fn tag(bias: u32, slot: Option<u32>, gi: Option<&GiCellTag>) -> u32 {
    (bias & 7) | slot.map_or(0, |s| ((s + 1) << 3) & LIGHT_SLOT_BITS) | gi.map_or(0, |g| g.0)
}

/// The tag bits the light-list slot may take (3–19); the GI cell sits
/// above them.
const LIGHT_SLOT_BITS: u32 = ((1 << crate::gi_tag::LOW_BITS) - 1) & !7;

type SlotQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        Option<&'static MeshLights>,
        Option<&'static ShadowBias>,
        Option<&'static GiCellTag>,
    ),
    Or<(Changed<MeshLights>, Changed<ShadowBias>, Changed<GiCellTag>)>,
>;

/// Writes the light lists of meshes whose lists changed and gives them
/// their tags.
fn assign_mesh_slots(mut commands: Commands, mut table: ResMut<LightTable>, meshes: SlotQuery) {
    for (entity, lights, bias, gi) in &meshes {
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
        let value = tag(bias.map_or(0, |b| b.0), slot, gi);
        commands.entity(entity).insert(MeshTag::new(value));
    }
}

/// Returns the slots of meshes that lost their list or were despawned.
fn free_mesh_slots(
    mut commands: Commands,
    mut table: ResMut<LightTable>,
    mut removed: RemovedComponents<MeshLights>,
    biases: Query<(Option<&ShadowBias>, Option<&GiCellTag>), Without<MeshLights>>,
) {
    for entity in removed.read() {
        let Some(s) = table.slot_of.remove(&entity) else {
            continue;
        };
        table.lists[s as usize] = [0; 8];
        table.free.push(s);
        table.dirty = true;
        // A mesh still alive keeps its bias level, lit by every light.
        if let Ok((bias, gi)) = biases.get(entity) {
            commands
                .entity(entity)
                .insert(MeshTag::new(tag(bias.map_or(0, |b| b.0), None, gi)));
        }
    }
}

fn upload(
    mut table: ResMut<LightTable>,
    sun: Res<crate::sun_fit::SunShadowFilter>,
    quality: Option<Res<crate::quality::ShadowQuality>>,
    split: Option<Res<crate::supersample::SplitTaps>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    if table.sun != *sun {
        table.sun = *sun;
        table.dirty = true;
    }
    let taps = sub_frame_taps(
        quality.map_or(crate::quality::ShadowQuality::default().0, |q| q.0),
        split.is_some_and(|s| s.0),
    );
    if table.shadow_samples != taps {
        table.shadow_samples = taps;
        table.dirty = true;
    }
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
    fn the_shadow_bias_scales_follow_the_render_flags() {
        assert_eq!(shadow_bias_scales(0), (1.0, 1.0));
        let (receive, cast) = shadow_bias_scales(3 << 16);
        assert!((receive - (1.0 - 3.0 * 0.005)).abs() < 1e-6);
        assert!((cast - (receive - 0.01)).abs() < 1e-6);
        let (receive, cast) = shadow_bias_scales(0x8_0000 | (2 << 16));
        assert_eq!(receive, cast);
        assert!((receive - (1.0 + 2.0 * 0.005)).abs() < 1e-6);
        // The inflated bit alone leaves the level at zero.
        assert_eq!(shadow_bias_scales(0x8_0000), (1.0, 1.0));
    }

    #[test]
    fn tags_pack_bias_and_slot() {
        assert_eq!(tag(3, None, None), 3);
        assert_eq!(tag(1, Some(0), None), 1 | 8);
        assert_eq!(tag(9, Some(4), None), 1 | 40);
        // The GI cell rides above the slot.
        let gi = GiCellTag(crate::gi_tag::pack(IVec2::new(2, 5)));
        let t = tag(1, Some(4), Some(&gi));
        assert_eq!(t & 0xF_FFFF, 1 | 40);
        assert_eq!(crate::gi_tag::unpack(t), Some(IVec2::new(2, 5)));
    }

    #[test]
    fn ids_ride_on_the_default_point_bias() {
        for id in [0, 1, 7, 228] {
            let bias = light_bias(id);
            assert_eq!(light_id_of(bias), id);
            // The field stays a valid Bevy bias, not an integer id: even a
            // whole level's ids add only a few thousandths.
            assert!(bias >= PointLight::DEFAULT_SHADOW_DEPTH_BIAS);
            assert!(bias <= PointLight::DEFAULT_SHADOW_DEPTH_BIAS + 0.01);
        }
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

    /// The first element carries the sun map's filter radius and caster
    /// polygon offset after the list index; with no fitted map all three
    /// are zero.
    #[test]
    fn element_zero_carries_the_sun_filter() {
        let data = layout(&LightTable::default());
        assert_eq!(&data[0][1..], &[0, 0, 0]);
        let table = LightTable {
            sun: crate::sun_fit::SunShadowFilter {
                radius: 0.000375,
                polygon_factor: 2.0,
                polygon_units: crate::sun_fit::DEPTH16_STEP,
            },
            ..default()
        };
        let data = layout(&table);
        assert_eq!(data[0][0], 2);
        assert_eq!(f32::from_bits(data[0][1]), 0.000375);
        assert_eq!(f32::from_bits(data[0][2]), 2.0);
        assert_eq!(f32::from_bits(data[0][3]), 1.0 / 65536.0);
    }

    /// `SdwSmps` per sub-frame for the settings: the shipped shadow quality
    /// 4 takes 2 taps a sub-frame supersampled (8 a frame), 8 without; the
    /// low qualities still take one a sub-frame, the highest 4 (16 a
    /// frame) and 16 without super-sampling.
    #[test]
    fn the_sub_frame_taps_follow_the_settings() {
        assert_eq!(sub_frame_taps(4, true), 2);
        assert_eq!(sub_frame_taps(4, false), 8);
        assert_eq!(sub_frame_taps(1, true), 1);
        assert_eq!(sub_frame_taps(1, false), 1);
        assert_eq!(sub_frame_taps(3, false), 4);
        assert_eq!(sub_frame_taps(5, true), 4);
        assert_eq!(sub_frame_taps(5, false), 16);
        // The top byte holds it above the first list's index.
        let table = LightTable {
            shadow_samples: 2,
            ..default()
        };
        assert_eq!(layout(&table)[0][0], 2 | 2 << HEADER_TAPS_SHIFT);
    }

    /// The live upload: the plugin writes the settings' `SdwSmps` into the
    /// buffer the surface shaders read, and rewrites it when the shadow
    /// quality or the super-sampling changes.
    #[test]
    fn the_upload_carries_the_settings_taps() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<ShaderBuffer>()
            .insert_resource(crate::quality::ShadowQuality(4))
            .insert_resource(crate::supersample::SplitTaps(true))
            .add_plugins(LightsPlugin);
        let header = |app: &App| {
            let buffers = app.world().resource::<Assets<ShaderBuffer>>();
            let buffer = buffers.get(AssetId::default()).expect("the light buffer");
            let bevy::render::storage::ShaderBufferData::Initialized { data, .. } = &buffer.data
            else {
                panic!("the light buffer holds its bytes");
            };
            u32::from_le_bytes(data[0..4].try_into().unwrap())
        };
        app.update();
        assert_eq!(header(&app) >> HEADER_TAPS_SHIFT, 2);
        app.insert_resource(crate::supersample::SplitTaps(false));
        app.update();
        assert_eq!(header(&app) >> HEADER_TAPS_SHIFT, 8);
        app.insert_resource(crate::quality::ShadowQuality(5));
        app.update();
        assert_eq!(header(&app) >> HEADER_TAPS_SHIFT, 16);
        assert_eq!(header(&app) & 0xFF_FFFF, 2);
    }
}
