//! The terrain blocks the original leaves out of every draw: those without a
//! composite.
//!
//! The original's lit terrain draw and its two shadow draws (all of a tile's
//! blocks into a cascade, one block into a light's cube face) walk the
//! inserted tiles' blocks and skip one whose composite state is blank: the
//! state the selector ([`super::composite::CompositeSelector`]) gives a new
//! block, a block that left the 44.4 m reach of the look-ahead point, and a
//! block that lost its set to a steal. So nothing past the reach is drawn or
//! casts a terrain shadow, and a block within reach shows from the frame its
//! first composite is written (a visible blank block sorts first, so while
//! sets are free every one is baked in the frame it comes into view, and a
//! visible block that finds no set stays undrawn). The selector runs live every
//! frame in [`super::live::drive_terrain_stream`]; this module applies its
//! answer to the draws:
//!
//! - [`mark_blank_blocks`] lists the blank blocks' entities once a frame,
//!   and the plant chunks standing on them,
//! - [`skip_blank_blocks`] takes both out of every camera's visible list
//!   after Bevy's visibility check (their view visibility and the live
//!   scene's drawn bit, which the selector reads back, are left as they
//!   were, so a blank block in view still sorts first),
//! - the shadow draws ([`super::shadow_draw`]) leave the blocks (not their
//!   plants) out of the sun's cascades and the cube faces rendered this
//!   frame.
//!
//! The plants follow the original's grass hooks: the main draw tests the
//! same blank bit per block node before drawing the node's plant instances,
//! so a blank block's plants are not drawn (nothing stands over the missing
//! ground); its sun and omni shadow draws do not test it, so the same plants
//! still cast (whether they cast at all is the shadow-quality rule of
//! `sync_plant_shadows`).
//!
//! This is the original's only draw path, so it holds for Exarust's default
//! per-pixel terrain as much as for the opt-in composite draw (DEC-6 is
//! about which shading the drawn blocks get, not about which blocks draw).
//! `EXA_TERRAIN_DRAW_BLANK=1` turns the rule off, to compare.
//! Rules: `docs/RENDERING.md` (Terrain blocks drawn).

use bevy::camera::visibility::VisibleEntities;
use bevy::ecs::entity::EntityHashSet;
use bevy::prelude::*;

use super::PlantBlock;
use super::live::TerrainStreaming;
use super::shadow_draw;

/// Which terrain blocks the draws take: the blank-block rule here and the
/// main-view rule of the shadow draws ([`super::shadow_draw`]), at their
/// places in the frame.
pub struct TerrainDrawRulesPlugin;

impl Plugin for TerrainDrawRulesPlugin {
    fn build(&self, app: &mut App) {
        use bevy::camera::visibility::VisibilitySystems;
        app.init_resource::<TerrainBlankRule>()
            .init_resource::<BlankTerrainBlocks>()
            .init_resource::<shadow_draw::TerrainShadowRule>()
            .init_resource::<shadow_draw::TerrainShadowStats>()
            .add_systems(
                PostUpdate,
                (
                    mark_blank_blocks
                        .in_set(exa_physics::FramePostPhase::Prepare)
                        .before(VisibilitySystems::CheckVisibility),
                    skip_blank_blocks
                        .in_set(exa_physics::FramePostPhase::View)
                        .after(crate::scene_live::cull_undrawn)
                        .after(mark_blank_blocks)
                        .after(VisibilitySystems::CheckVisibility)
                        .before(bevy::light::SimulationLightSystems::CheckLightVisibility)
                        .before(VisibilitySystems::MarkNewlyHiddenEntitiesInvisible),
                    shadow_draw::sun_casts_drawn_blocks
                        .in_set(exa_physics::FramePostPhase::View)
                        .after(mark_blank_blocks)
                        .after(bevy::light::SimulationLightSystems::CheckLightVisibility),
                    shadow_draw::cube_faces_skip_undrawn_blocks
                        .in_set(exa_physics::FramePostPhase::View)
                        .after(mark_blank_blocks)
                        .after(crate::scene_live::run_scene)
                        .before(exa_render::lights::LightsSystems::Cubes),
                ),
            );
    }
}

/// Whether blank blocks are left out of the draws (the original's rule;
/// `EXA_TERRAIN_DRAW_BLANK=1` turns it off).
#[derive(Resource, Clone, Copy, Debug)]
pub struct TerrainBlankRule {
    pub skip: bool,
}

impl Default for TerrainBlankRule {
    fn default() -> Self {
        Self {
            skip: !std::env::var("EXA_TERRAIN_DRAW_BLANK").is_ok_and(|v| v == "1"),
        }
    }
}

/// This frame's blank terrain blocks and the plant chunks on them (empty
/// when the rule is off or no terrain is loaded), and how many entries the
/// last frame took out of the cameras' visible lists.
#[derive(Resource, Clone, Debug, Default)]
pub struct BlankTerrainBlocks {
    pub blocks: EntityHashSet,
    /// The plant chunks of the blank blocks: left out of the camera draws,
    /// kept among the shadow casters.
    pub plants: EntityHashSet,
    /// Blocks taken out of the cameras' lists last frame (summed over the
    /// cameras).
    pub left_out: usize,
    /// Plant chunks taken out of the cameras' lists last frame.
    pub plants_left_out: usize,
    frames: u64,
}

impl BlankTerrainBlocks {
    /// Whether `entity` is a blank block the draws leave out.
    pub fn is_blank(&self, entity: Entity) -> bool {
        self.blocks.contains(&entity)
    }
}

/// Lists the blocks whose composite state is blank after this frame's
/// selection, and the plant chunks standing on them.
pub fn mark_blank_blocks(
    rule: Res<TerrainBlankRule>,
    streaming: Option<Res<TerrainStreaming>>,
    chunks: Query<(Entity, &PlantBlock)>,
    mut blank: ResMut<BlankTerrainBlocks>,
) {
    blank.blocks.clear();
    blank.plants.clear();
    let Some(streaming) = streaming.filter(|_| rule.skip) else {
        return;
    };
    blank.blocks.extend(streaming.blank_entities());
    if blank.blocks.is_empty() {
        return;
    }
    let BlankTerrainBlocks { blocks, plants, .. } = &mut *blank;
    plants.extend(
        chunks
            .iter()
            .filter(|(_, block)| blocks.contains(&block.0))
            .map(|(chunk, _)| chunk),
    );
}

/// Takes the blank blocks and their plant chunks out of every camera's
/// visible mesh list, as the original's lit terrain draw and its grass draw
/// skip them in every renderer they are hooked into.
///
/// Every 240 frames it logs the counts; with `EXA_TERRAIN_DRAW_DEBUG=1` the
/// log also names each camera that listed a blank block (to find a view
/// that lists more than it should) and counts the plant chunks that name no
/// block (they would draw whatever their ground does).
pub fn skip_blank_blocks(
    mut blank: ResMut<BlankTerrainBlocks>,
    mut cameras: Query<(Entity, &mut VisibleEntities, &Camera, Option<&Name>)>,
    unlinked: Query<(), (With<super::PlantChunk>, Without<PlantBlock>)>,
    mut debug: Local<Option<bool>>,
) {
    if blank.blocks.is_empty() {
        blank.left_out = 0;
        blank.plants_left_out = 0;
        return;
    }
    let debug = *debug
        .get_or_insert_with(|| std::env::var("EXA_TERRAIN_DRAW_DEBUG").is_ok_and(|v| v == "1"));
    let logging = blank.frames.is_multiple_of(240);
    let mut per_camera = Vec::new();
    let (mut left_out, mut plants_left_out) = (0, 0);
    for (entity, mut visible, camera, name) in &mut cameras {
        let list = visible.get_mut(std::any::TypeId::of::<Mesh3d>());
        let before = list.len();
        list.retain(|e| !blank.blocks.contains(e));
        let blocks = before - list.len();
        let kept = list.len();
        list.retain(|e| !blank.plants.contains(e));
        let plants = kept - list.len();
        left_out += blocks;
        plants_left_out += plants;
        if debug && logging && blocks + plants > 0 {
            per_camera.push(format!(
                "{entity} {:?} order {} active {}: {blocks} blocks, {plants} plants",
                name.map_or("(unnamed)", |n| n.as_str()),
                camera.order,
                camera.is_active,
            ));
        }
    }
    blank.left_out = left_out;
    blank.plants_left_out = plants_left_out;
    if logging {
        info!(
            "terrain blank blocks: {} without a composite, {left_out} camera draws left out, \
             {plants_left_out} plant chunk draws left out",
            blank.blocks.len()
        );
        for line in per_camera {
            info!("terrain blank blocks by camera: {line}");
        }
        if debug {
            info!(
                "terrain blank blocks: {} plant chunks name no block",
                unlinked.iter().count()
            );
        }
    }
    blank.frames += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::TerrainBlock;
    use crate::terrain::composite::{CompositeFeatures, CompositePools, detail_size};
    use crate::terrain::live::{LiveBlock, LiveTile, drive_terrain_stream};
    use crate::terrain::stream::{TerrainStream, TileStream};
    use bevy::camera::visibility::{CascadesVisibleEntities, VisibleMeshEntities};
    use exa_render::cube_shadows::CubeCasterGate;

    /// A headless app with the draw rules and the live terrain driver, a
    /// camera at the origin looking down −Z (its look-ahead point is 10 m
    /// behind it, the block centre (3, 9)), and three blocks: two in reach
    /// of that point and one 61 m from it.
    fn app(rule: bool) -> (App, Entity, [Entity; 3]) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(TerrainDrawRulesPlugin)
            .insert_resource(TerrainBlankRule { skip: rule })
            .init_resource::<CubeCasterGate>()
            .add_systems(Update, drive_terrain_stream);
        let world = app.world_mut();
        let centres = [
            Vec2::new(0.0, 0.0),
            Vec2::new(6.0, 0.0),
            Vec2::new(0.0, 70.0),
        ];
        let entities = centres.map(|c| {
            world
                .spawn(TerrainBlock {
                    bx: (c.x / 6.0) as u32,
                    by: (c.y / 6.0) as u32,
                    level: TerrainBlock::LEVEL,
                    room: None,
                })
                .id()
        });
        let tile = world.spawn_empty().id();
        let blocks: Vec<LiveBlock> = centres
            .iter()
            .zip(entities)
            .map(|(&centre, entity)| LiveBlock {
                entity,
                tile: 0,
                centre,
                half: Vec3::new(3.0, TerrainBlock::HALF_HEIGHT, 3.0),
            })
            .collect();
        world.insert_resource(super::super::live::TerrainStreaming::new(
            TerrainStream::new(vec![TileStream::raw()]),
            CompositePools::new(detail_size(3), CompositeFeatures::default()),
            vec![Some(LiveTile {
                entity: tile,
                blocks: entities.to_vec(),
            })],
            blocks,
        ));
        let mut visible = VisibleEntities::default();
        visible
            .get_mut(std::any::TypeId::of::<Mesh3d>())
            .extend(entities);
        let camera = world
            .spawn((Camera::default(), GlobalTransform::IDENTITY, visible))
            .id();
        // A sun whose one cascade lists every block for the camera.
        let mut cascades = CascadesVisibleEntities::default();
        cascades.entities.insert(
            camera,
            vec![VisibleMeshEntities {
                entities: entities.to_vec(),
            }],
        );
        world.spawn(cascades);
        (app, camera, entities)
    }

    fn cast(app: &mut App) -> Vec<Entity> {
        let mut suns = app.world_mut().query::<&CascadesVisibleEntities>();
        let sun = suns.single(app.world()).unwrap();
        sun.entities.values().next().unwrap()[0].entities.clone()
    }

    fn drawn(app: &mut App, camera: Entity) -> Vec<Entity> {
        app.world_mut()
            .get_mut::<VisibleEntities>(camera)
            .unwrap()
            .get_mut(std::any::TypeId::of::<Mesh3d>())
            .clone()
    }

    #[test]
    fn a_live_frame_draws_and_casts_only_the_composited_blocks() {
        let (mut app, camera, [near, side, far]) = app(true);
        app.update();
        // The selection baked the two blocks in reach (visible blank blocks
        // sort first and are all baked at once); the far one stays blank.
        let streaming = app
            .world()
            .resource::<super::super::live::TerrainStreaming>();
        let blank: Vec<bool> = streaming
            .selector()
            .blocks()
            .iter()
            .map(|b| b.is_blank())
            .collect();
        assert_eq!(blank, [false, false, true]);
        let marked = app.world().resource::<BlankTerrainBlocks>();
        assert!(marked.is_blank(far) && !marked.is_blank(near) && !marked.is_blank(side));
        assert_eq!(marked.left_out, 1);
        // The camera draws the two composited blocks only, and the cube
        // faces leave the blank one out too.
        assert_eq!(drawn(&mut app, camera), [near, side]);
        assert_eq!(cast(&mut app), [near, side]);
        let gate = app.world().resource::<CubeCasterGate>();
        assert!(gate.left_out.contains(&far));
        assert!(!gate.left_out.contains(&near) && !gate.left_out.contains(&side));
    }

    #[test]
    fn with_the_rule_off_every_block_draws() {
        let (mut app, camera, entities) = app(false);
        app.update();
        assert!(
            app.world()
                .resource::<BlankTerrainBlocks>()
                .blocks
                .is_empty()
        );
        assert_eq!(drawn(&mut app, camera), entities);
        assert_eq!(cast(&mut app), entities);
        assert!(app.world().resource::<CubeCasterGate>().left_out.is_empty());
    }

    /// Plant chunks on the near and the far block, listed by the camera and
    /// by the sun's cascade.
    fn with_plants(app: &mut App, camera: Entity, near: Entity, far: Entity) -> [Entity; 2] {
        let world = app.world_mut();
        let chunks = [near, far].map(|block| world.spawn(PlantBlock(block)).id());
        world
            .get_mut::<VisibleEntities>(camera)
            .unwrap()
            .get_mut(std::any::TypeId::of::<Mesh3d>())
            .extend(chunks);
        let mut suns = world.query::<&mut CascadesVisibleEntities>();
        let mut sun = suns.single_mut(world).unwrap();
        sun.entities.values_mut().next().unwrap()[0]
            .entities
            .extend(chunks);
        chunks
    }

    #[test]
    fn a_blank_blocks_plants_cast_but_are_not_drawn() {
        let (mut app, camera, [near, side, far]) = app(true);
        let [near_plants, far_plants] = with_plants(&mut app, camera, near, far);
        app.update();
        let marked = app.world().resource::<BlankTerrainBlocks>();
        assert!(marked.plants.contains(&far_plants) && !marked.plants.contains(&near_plants));
        assert_eq!((marked.left_out, marked.plants_left_out), (1, 1));
        // The camera draws the composited block's plants only; the sun's
        // cascade keeps both chunks (the grass shadow draws do not test the
        // blank bit) and leaves the blank block out.
        assert_eq!(drawn(&mut app, camera), [near, side, near_plants]);
        assert_eq!(cast(&mut app), [near, side, near_plants, far_plants]);
        // The cube faces gate blocks only.
        let gate = app.world().resource::<CubeCasterGate>();
        assert!(!gate.left_out.contains(&far_plants));
    }

    #[test]
    fn with_the_rule_off_every_plant_chunk_draws() {
        let (mut app, camera, [near, side, far]) = app(false);
        let [near_plants, far_plants] = with_plants(&mut app, camera, near, far);
        app.update();
        assert!(
            app.world()
                .resource::<BlankTerrainBlocks>()
                .plants
                .is_empty()
        );
        assert_eq!(
            drawn(&mut app, camera),
            [near, side, far, near_plants, far_plants]
        );
    }

    #[test]
    fn without_a_terrain_nothing_is_blank() {
        let (mut app, camera, entities) = app(true);
        app.world_mut()
            .remove_resource::<super::super::live::TerrainStreaming>();
        app.update();
        assert!(
            app.world()
                .resource::<BlankTerrainBlocks>()
                .blocks
                .is_empty()
        );
        assert_eq!(drawn(&mut app, camera), entities);
    }
}
