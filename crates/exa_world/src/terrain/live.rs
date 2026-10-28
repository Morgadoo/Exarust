//! The static terrain's live side after a level loads: the frame hook's tile
//! walk, the composite selection that runs once a frame against the camera,
//! and the nearest-light rank of the blocks that are out of view.
//!
//! The original keeps its tiles in the scene after the block-until-loaded
//! wait and then runs two per-frame passes: the frame hook ([`stream`]) that
//! inserts, uploads or unloads at most one tile, and the block selection
//! ([`composite`]) that ranks every 6 m block and bakes one composite a
//! frame. Exarust layers the materials per pixel instead of baking sets, so
//! the selection drives the block ranks, the set pools and the per-block
//! composite state; its set assignments are handed to
//! [`exa_render::terrain_composite`] when a host is present.
//!
//! Evidence: REA on the priority slot, the block selector and the frame
//! hook; the numbers and orders are pinned in `stream` and `composite`.
//! The light volume's planes are a guess: the pack shows the distance
//! arithmetic and the plane list, not what fills the list, so Exarust
//! models a point light's reach as a box (see [`super::stream::LightVolume`]).

use bevy::camera::visibility::ViewVisibility;
use bevy::prelude::*;

use super::composite::{BlockComposite, CompositePools, CompositeSelector, FAR_D2};
use super::stream::{
    LightVolume, OrientedBox, TerrainStream, TileAction, composite_focus, nearest_light_distance,
};

/// One spawned tile of the terrain: its entity and the block entities under
/// it, in the block order the selector uses.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveTile {
    pub entity: Entity,
    pub blocks: Vec<Entity>,
}

/// One spawned 6 m block: its entity, its centre on the X/Z plane (metres,
/// world) and its half extents (metres, the flat 10 cm slab).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiveBlock {
    pub entity: Entity,
    /// The tile it belongs to.
    pub tile: usize,
    pub centre: Vec2,
    pub half: Vec3,
}

impl LiveBlock {
    /// The block's oriented box in centimetres, as the light-distance test
    /// takes it.
    pub fn volume(&self) -> OrientedBox {
        OrientedBox::axis_aligned(
            Vec3::new(self.centre.x * 100.0, 0.0, self.centre.y * 100.0),
            Vec3::new(
                self.half.x * 100.0,
                self.half.y * 100.0,
                self.half.z * 100.0,
            ),
        )
    }
}

/// What one frame's selection produced.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TerrainTick {
    /// The priority base the frame's ring was set to.
    pub base: u32,
    /// The selection baked (or copied) something this frame.
    pub changed: bool,
    /// The number of selected blocks after the sort.
    pub selected: usize,
    /// The blocks whose composite was written, with the set each holds.
    pub baked: Vec<(usize, u32)>,
}

/// The terrain's live streaming and composite state, inserted when a level's
/// terrain finishes loading and dropped with the level.
#[derive(Resource)]
pub struct TerrainStreaming {
    stream: TerrainStream,
    selector: CompositeSelector,
    tiles: Vec<Option<LiveTile>>,
    blocks: Vec<LiveBlock>,
    /// The selector indices belonging to each tile, from the blocks' tile.
    block_indices: Vec<Vec<usize>>,
    focus: Vec2,
    frames: u64,
}

impl TerrainStreaming {
    /// The live state over the tiles, blocks and streaming bits a finished
    /// terrain build left.
    pub fn new(
        stream: TerrainStream,
        pools: CompositePools,
        tiles: Vec<Option<LiveTile>>,
        blocks: Vec<LiveBlock>,
    ) -> Self {
        let blocks_for_selector = blocks
            .iter()
            .map(|block| BlockComposite::new(block.centre))
            .collect();
        let mut block_indices = vec![Vec::new(); tiles.len()];
        for (index, block) in blocks.iter().enumerate() {
            if let Some(list) = block_indices.get_mut(block.tile) {
                list.push(index);
            }
        }
        Self {
            stream,
            selector: CompositeSelector::new(blocks_for_selector, pools),
            tiles,
            blocks,
            block_indices,
            focus: Vec2::ZERO,
            frames: 0,
        }
    }

    /// Marks every block of `tile` as wanting a composite, as the frame
    /// hook does when it inserts a tile.
    pub fn want_tile_blocks(&mut self, tile: usize) {
        for &index in self.block_indices.get(tile).into_iter().flatten() {
            if let Some(block) = self.selector.block_mut(index) {
                block.want();
            }
        }
    }

    /// The selector indices of a tile's blocks.
    pub fn tile_blocks(&self, tile: usize) -> &[usize] {
        self.block_indices.get(tile).map_or(&[], Vec::as_slice)
    }

    /// Takes a tile's blocks out of the selection and hands their sets back,
    /// as the frame hook's unload does.
    pub fn drop_tile_blocks(&mut self, tile: usize) {
        let blocks = self.block_indices.get(tile).cloned().unwrap_or_default();
        self.selector.drop_blocks(&blocks);
    }

    pub fn stream(&self) -> &TerrainStream {
        &self.stream
    }

    pub fn selector(&self) -> &CompositeSelector {
        &self.selector
    }

    pub fn blocks(&self) -> &[LiveBlock] {
        &self.blocks
    }

    pub fn tiles(&self) -> &[Option<LiveTile>] {
        &self.tiles
    }

    /// The block centre the last frame measured from, metres.
    pub fn focus(&self) -> Vec2 {
        self.focus
    }

    /// How many frames the live pass has run.
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// One frame: take the visibility and nearest-light distances the
    /// caller measured, rank the blocks from `focus` and run the selection:
    /// cut-offs, sort, set hand-out and the one-block-a-frame bake.
    pub fn tick(&mut self, focus: Vec2, visible: &[bool], light: &[Option<f32>]) -> TerrainTick {
        self.focus = focus;
        self.frames += 1;
        for index in 0..self.blocks.len() {
            let shown = visible.get(index).copied().unwrap_or(true);
            let distance = light.get(index).copied().flatten();
            if let Some(block) = self.selector.block_mut(index) {
                block.visible = shown;
                block.light_distance = distance;
            }
        }
        let base = self.selector.rank(focus);
        let changed = self.selector.reselect(focus);
        let baked = self
            .selector
            .last_baked()
            .iter()
            .filter_map(|&index| {
                self.selector
                    .blocks()
                    .get(index)
                    .and_then(|block| block.set().map(|set| (index, set)))
            })
            .collect();
        TerrainTick {
            base,
            changed,
            selected: self.selector.selected().len(),
            baked,
        }
    }

    /// The frame hook's one-tile step after the load, with the entity the
    /// insert or unload applies to.
    fn entity_for(&self, tile: usize) -> Option<Entity> {
        self.tiles
            .get(tile)
            .and_then(|tile| tile.as_ref())
            .map(|t| t.entity)
    }
}

/// The camera the composite focus follows: the active camera with the
/// lowest order, as the original's main view.
fn view_camera(cameras: &Query<(&Camera, &GlobalTransform)>) -> Option<(Vec2, Vec2)> {
    cameras
        .iter()
        .filter(|(camera, _)| camera.is_active)
        .min_by_key(|(camera, _)| camera.order)
        .map(|(_, transform)| {
            let back = transform.back();
            (
                Vec2::new(transform.translation().x, transform.translation().z),
                Vec2::new(back.x, back.z),
            )
        })
}

/// The level's lights as the reach volumes the block rank tests. A light's
/// stored radius (the GI radius, which an unranged light keeps) is the reach;
/// without one, the point light's range stands in. (Guess: see the module
/// note.)
fn light_volumes(
    lights: &Query<(
        &GlobalTransform,
        Option<&exa_render::gi::GiRadius>,
        Option<&PointLight>,
    )>,
) -> Vec<LightVolume> {
    const REACH: f32 = 1.25;
    let mut volumes = Vec::new();
    for (transform, radius, point) in lights {
        let reach = radius
            .map(|radius| radius.0 * REACH)
            .or_else(|| point.map(|point| point.range * 100.0))
            .unwrap_or(0.0);
        if reach.is_nan() || reach <= 0.0 || !reach.is_finite() {
            continue;
        }
        let centre = transform.translation() * 100.0;
        volumes.push(LightVolume::box_volume(centre, Vec3::splat(reach)));
    }
    volumes
}

/// The nearest-light distance of every block that is hidden and within the
/// composite reach, centimetres; the other blocks answer `None`. This is the
/// only case the original measures a light distance for (a visible block
/// sorts by its composite ring instead).
pub fn block_light_distances(
    focus: Vec2,
    blocks: &[LiveBlock],
    visible: &[bool],
    lights: &[LightVolume],
) -> Vec<Option<f32>> {
    blocks
        .iter()
        .enumerate()
        .map(|(index, block)| {
            let shown = visible.get(index).copied().unwrap_or(true);
            let (dx, dz) = (
                (focus.x - block.centre.x) * 100.0,
                (focus.y - block.centre.y) * 100.0,
            );
            let d2 = dz * dz + dx * dx;
            let in_reach = d2 <= FAR_D2 || d2.is_nan();
            if shown || !in_reach || lights.is_empty() {
                return None;
            }
            Some(nearest_light_distance(lights.iter(), &block.volume()))
        })
        .collect()
}

/// The per-frame terrain driver: the frame hook's tile walk, then the
/// composite selection around the camera's look-ahead point.
pub fn drive_terrain_stream(
    mut commands: Commands,
    streaming: Option<ResMut<TerrainStreaming>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    blocks: Query<&ViewVisibility>,
    lights: Query<(
        &GlobalTransform,
        Option<&exa_render::gi::GiRadius>,
        Option<&PointLight>,
    )>,
    host: Option<ResMut<exa_render::terrain_composite::TerrainCompositeHost>>,
    assets: Option<ResMut<Assets<Image>>>,
) {
    let Some(mut streaming) = streaming else {
        return;
    };
    // The frame hook, at most one tile; an inserted or ready tile marks its
    // blocks as wanting a composite.
    if let Some(step) = streaming.stream.frame() {
        for tile in step.wanted.clone() {
            streaming.want_tile_blocks(tile);
        }
        if let Some((tile, action)) = step.handled
            && let Some(entity) = streaming.entity_for(tile)
        {
            match action {
                TileAction::Insert => {
                    commands.entity(entity).insert(Visibility::Visible);
                }
                TileAction::Unload => {
                    streaming.drop_tile_blocks(tile);
                    commands.entity(entity).insert(Visibility::Hidden);
                }
                TileAction::Upload | TileAction::None => {}
            }
        }
    }
    let Some((camera, back)) = view_camera(&cameras) else {
        return;
    };
    let focus = composite_focus(camera, back);
    let visible: Vec<bool> = streaming
        .blocks
        .iter()
        .map(|block| {
            blocks
                .get(block.entity)
                .map(|visibility| visibility.get())
                .unwrap_or(true)
        })
        .collect();
    // A hidden, in-reach block ranks by its nearest light; the volumes are
    // built only then, so a view where every block is visible costs nothing.
    let distances = if visible.iter().any(|shown| !shown) {
        let volumes = light_volumes(&lights);
        block_light_distances(focus, &streaming.blocks, &visible, &volumes)
    } else {
        vec![None; streaming.blocks.len()]
    };
    let tick = streaming.tick(focus, &visible, &distances);
    // Hand the sets the selection holds to the renderer's composite host.
    // One new set a frame: making many render targets at once would stall
    // the frame, and nothing draws them yet (the per-pixel shader does).
    if let (Some(mut host), Some(mut assets)) = (host, assets) {
        use exa_render::terrain_composite::SetFeatures;
        let held: Vec<(u32, i32, SetFeatures)> = streaming
            .selector()
            .blocks()
            .iter()
            .filter_map(|block| block.set())
            .filter_map(|set| {
                let record = streaming.selector().pools().set(set)?;
                Some((
                    set,
                    record.level,
                    SetFeatures {
                        normal: record.normal,
                        gloss: record.gloss,
                        displacement: record.displacement,
                    },
                ))
            })
            .collect();
        let _ = host.sync(&mut assets, &held, 1);
    }
    let _ = tick;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::composite::{CompositeFeatures, CompositePools, detail_size};
    use crate::terrain::stream::{LightVolume, TileStream, light_sort_key};

    fn entity(raw: u32) -> Entity {
        Entity::from_raw_u32(raw).unwrap()
    }

    fn block(tile: usize, x: f32, z: f32) -> LiveBlock {
        LiveBlock {
            entity: entity(100 + x as u32 * 10 + z as u32),
            tile,
            centre: Vec2::new(x, z),
            half: Vec3::new(3.0, 0.1, 3.0),
        }
    }

    fn streaming(centres: &[(f32, f32)]) -> TerrainStreaming {
        let blocks: Vec<LiveBlock> = centres
            .iter()
            .enumerate()
            .map(|(i, &(x, z))| LiveBlock {
                entity: entity(i as u32 + 1),
                tile: 0,
                centre: Vec2::new(x, z),
                half: Vec3::new(3.0, 0.1, 3.0),
            })
            .collect();
        let stream = TerrainStream::new(vec![TileStream::raw()]);
        let pools = CompositePools::new(detail_size(3), CompositeFeatures::default());
        TerrainStreaming::new(
            stream,
            pools,
            vec![Some(LiveTile {
                entity: entity(99),
                blocks: blocks.iter().map(|b| b.entity).collect(),
            })],
            blocks,
        )
    }

    #[test]
    fn a_live_tick_ranks_and_bakes_the_blank_blocks() {
        let mut streaming = streaming(&[(0.0, 0.0), (6.0, 0.0), (30.0, 0.0), (50.0, 0.0)]);
        let visible = [true; 4];
        let tick = streaming.tick(Vec2::ZERO, &visible, &[None; 4]);
        // The selected list starts empty, so the priority slot ranks nothing
        // and the base is 6000; every baked block then takes the ring it was
        // baked in. The out-of-reach block keeps its unranked priority.
        assert_eq!(tick.base, 6000);
        assert!(
            streaming
                .selector()
                .blocks()
                .iter()
                .all(|b| b.distance == 0)
        );
        assert_eq!(
            streaming
                .selector()
                .blocks()
                .iter()
                .map(|b| b.priority)
                .collect::<Vec<_>>(),
            [6000, 6000, 6000, 0]
        );
        // Three blocks are in reach; the first two bake high, the third low,
        // and the frame's ring advances once.
        assert_eq!(tick.selected, 3);
        assert!(tick.changed);
        assert_eq!(streaming.selector().ring(), 6001);
        assert_eq!(
            tick.baked.iter().map(|&(i, _)| i).collect::<Vec<_>>(),
            [0, 1, 2]
        );
        assert_eq!(streaming.selector().blocks()[0].state(), 0xa00);
        assert_eq!(streaming.selector().blocks()[1].state(), 0xa00);
        assert_eq!(streaming.selector().blocks()[2].state(), 0x500);
        assert_eq!(
            (
                streaming.selector().pools().high_free(),
                streaming.selector().pools().low_free()
            ),
            (19, 42)
        );
        assert_eq!(streaming.frames(), 1);
        assert_eq!(streaming.focus(), Vec2::ZERO);
    }

    #[test]
    fn the_priority_slot_ranks_only_the_selected_list() {
        let mut streaming = streaming(&[(0.0, 0.0), (6.0, 0.0), (30.0, 0.0), (50.0, 0.0)]);
        let visible = [true; 4];
        streaming.tick(Vec2::ZERO, &visible, &[None; 4]);
        // The next frame ranks the three selected blocks: 6000 − d each, and
        // they are not wanted, so their priority follows the base.
        let tick = streaming.tick(Vec2::ZERO, &visible, &[None; 4]);
        assert_eq!(tick.base, 6000);
        assert_eq!(
            streaming
                .selector()
                .blocks()
                .iter()
                .map(|b| b.distance)
                .collect::<Vec<_>>(),
            [6000, 5400, 3000, 0]
        );
        assert_eq!(
            streaming
                .selector()
                .blocks()
                .iter()
                .map(|b| b.priority)
                .collect::<Vec<_>>(),
            [0, 600, 3000, 0]
        );
        // The near blocks sort first among the visible, non-blank blocks.
        let keys: Vec<u32> = streaming
            .selector()
            .selected()
            .iter()
            .map(|&i| streaming.selector().blocks()[i].sort_key())
            .collect();
        assert!(keys[0] < keys[1] && keys[1] < keys[2], "{keys:?}");
    }

    #[test]
    fn a_quiet_frame_resets_the_ring_to_the_priority_base() {
        let mut streaming = streaming(&[(0.0, 0.0), (6.0, 0.0)]);
        let visible = [true; 2];
        assert!(streaming.tick(Vec2::ZERO, &visible, &[None; 2]).changed);
        assert_eq!(streaming.selector().ring(), 6001);
        // The baked blocks no longer want work, so the base is 6000 again and
        // the ring follows it, with no bake.
        let tick = streaming.tick(Vec2::ZERO, &visible, &[None; 2]);
        assert_eq!(tick.base, 6000);
        assert_eq!(streaming.selector().ring(), 6000);
        assert!(!tick.changed);
        assert!(tick.baked.is_empty());
        assert_eq!(tick.selected, 2);
    }

    #[test]
    fn blocks_past_the_reach_leave_and_hand_their_sets_back() {
        let mut streaming = streaming(&[(0.0, 0.0), (6.0, 0.0)]);
        let visible = [true; 2];
        streaming.tick(Vec2::ZERO, &visible, &[None; 2]);
        assert_eq!(streaming.selector().pools().high_free(), 19);
        let tick = streaming.tick(Vec2::new(200.0, 0.0), &visible, &[None; 2]);
        assert_eq!(tick.selected, 0);
        assert!(streaming.selector().selected().is_empty());
        assert_eq!(streaming.selector().pools().high_free(), 21);
        assert_eq!(streaming.selector().blocks()[0].state(), 6);
    }

    #[test]
    fn hidden_blocks_rank_by_their_nearest_light() {
        let mut streaming = streaming(&[(0.0, 0.0), (6.0, 0.0)]);
        // Bake both so they are no longer blank.
        streaming.tick(Vec2::ZERO, &[true; 2], &[None; 2]);
        // A light at (10 m, 0) with a 2 m reach; block 0 is 10 m away and
        // block 1 is 4 m away, so their clearances differ.
        let lights = [LightVolume::box_volume(
            Vec3::new(1000.0, 0.0, 0.0),
            Vec3::splat(200.0),
        )];
        let visible = [false; 2];
        let distances = block_light_distances(Vec2::ZERO, streaming.blocks(), &visible, &lights);
        assert_eq!(distances[0], Some(1000.0 - 200.0 - 300.0));
        // The second block's 3 m half extent reaches the light's box, so its
        // clearance is 0.
        assert_eq!(distances[1], Some(0.0));
        let tick = streaming.tick(Vec2::ZERO, &visible, &distances);
        let keys: Vec<u32> = streaming
            .selector()
            .blocks()
            .iter()
            .map(|b| b.sort_key())
            .collect();
        assert_eq!(keys[0], light_sort_key(500.0));
        assert_eq!(keys[1], light_sort_key(0.0));
        // A visible block never measures one.
        let shown = block_light_distances(Vec2::ZERO, streaming.blocks(), &[true; 2], &lights);
        assert_eq!(shown, [None, None]);
        // A block past the 44.4 m reach is not measured either.
        let far = block_light_distances(
            Vec2::new(100.0, 0.0),
            streaming.blocks(),
            &[false; 2],
            &lights,
        );
        assert_eq!(far, [None, None]);
        let _ = tick;
    }

    #[test]
    fn inserting_a_tile_marks_its_blocks_wanted() {
        let blocks = vec![
            LiveBlock {
                entity: entity(1),
                tile: 0,
                centre: Vec2::ZERO,
                half: Vec3::new(3.0, 0.1, 3.0),
            },
            LiveBlock {
                entity: entity(2),
                tile: 0,
                centre: Vec2::new(6.0, 0.0),
                half: Vec3::new(3.0, 0.1, 3.0),
            },
            LiveBlock {
                entity: entity(3),
                tile: 1,
                centre: Vec2::new(30.0, 0.0),
                half: Vec3::new(3.0, 0.1, 3.0),
            },
        ];
        let stream = TerrainStream::new(vec![TileStream::raw(); 2]);
        let pools = CompositePools::new(detail_size(3), CompositeFeatures::default());
        let mut streaming = TerrainStreaming::new(stream, pools, vec![None; 2], blocks);
        assert_eq!(streaming.tile_blocks(0), [0, 1]);
        assert_eq!(streaming.tile_blocks(1), [2]);
        let visible = [true; 3];
        assert!(streaming.tick(Vec2::ZERO, &visible, &[None; 3]).changed);
        assert!(
            streaming
                .selector()
                .blocks()
                .iter()
                .all(|block| block.state() & 2 == 0)
        );
        streaming.want_tile_blocks(0);
        assert!(
            streaming
                .selector()
                .blocks()
                .iter()
                .take(2)
                .all(|block| block.state() & 2 != 0)
        );
        assert_eq!(streaming.selector().blocks()[2].state() & 2, 0);
        assert!(streaming.tick(Vec2::ZERO, &visible, &[None; 3]).changed);
    }

    #[test]
    fn unloading_a_tile_hands_its_blocks_sets_back() {
        let mut streaming = streaming(&[(0.0, 0.0), (6.0, 0.0)]);
        let visible = [true; 2];
        assert!(streaming.tick(Vec2::ZERO, &visible, &[None; 2]).changed);
        assert_eq!(streaming.selector().pools().high_free(), 19);
        streaming.drop_tile_blocks(0);
        assert!(streaming.selector().selected().is_empty());
        assert_eq!(streaming.selector().pools().high_free(), 21);
        assert_eq!(streaming.selector().blocks()[0].state(), 6);
        assert!(streaming.selector().blocks()[0].set().is_none());
        // Dropping a tile that has no blocks is harmless.
        streaming.drop_tile_blocks(7);
    }

    #[test]
    fn the_live_blocks_share_the_terrain_blocks_slab() {
        let block = block(0, 6.0, -3.0);
        let obb = block.volume();
        assert_eq!(obb.centre, Vec3::new(600.0, 0.0, -300.0));
        assert_eq!(obb.half_extents, Vec3::new(300.0, 10.0, 300.0));
    }

    #[test]
    fn the_block_until_loaded_wait_inserts_every_built_tile() {
        let mut stream = TerrainStream::new(vec![TileStream::raw(); 3]);
        for tile in &mut stream.tiles {
            tile.mark_built(false);
        }
        let mut actions = Vec::new();
        let calls = stream.wait_until_inserted(|tile, action| actions.push((tile, action)));
        assert!(stream.all_inserted());
        assert_eq!(
            actions,
            [
                (0, TileAction::Insert),
                (1, TileAction::Insert),
                (2, TileAction::Insert)
            ]
        );
        assert!(calls >= 3);
        // Nothing more to do on later frames.
        assert!(stream.all_inserted());
        assert_eq!(stream.frame().unwrap().handled, None);
    }

    #[test]
    fn a_refreshed_terrain_counts_its_delay_down() {
        let mut stream = TerrainStream::new(vec![TileStream::raw(); 2]);
        for tile in &mut stream.tiles {
            tile.mark_built(false);
        }
        stream.wait_until_inserted(|_, _| {});
        assert!(stream.all_inserted());
        stream.refresh_all();
        assert_eq!(stream.delay, 1);
        assert!(!stream.all_inserted());
        assert_eq!(stream.frame(), None);
        assert_eq!(stream.delay, 0);
        // The refreshed tiles are inserted again.
        let mut actions = Vec::new();
        stream.wait_until_inserted(|tile, action| actions.push((tile, action)));
        assert_eq!(actions.len(), 2);
    }
}
