//! Baking the selected blocks into their composite sets and drawing from
//! them, the original's way of showing its terrain.
//!
//! The selector ([`super::composite::CompositeSelector`]) hands each block
//! within reach a set, the renderer hosts the set's targets
//! ([`exa_render::terrain_composite::TerrainCompositeHost`]), and this
//! module bakes a holder's layers into its set
//! ([`exa_render::terrain_composite::TerrainBakeQueue`], one block a frame)
//! and, once the bake has run, gives the block its own material that reads
//! the set (`terrain.wesl`'s composite path, the original's `shadetrn`
//! program with its block's `TexPos`). A block that loses its set, or whose
//! set's targets are rebuilt, goes back to its tile's per-pixel material.
//!
//! All of it runs only when the host is enabled (`EXA_TERRAIN_SETS=1`), so
//! the default path keeps the per-pixel draw and costs nothing here.
//!
//! The original's draw binds a block's set whenever the block is drawn and
//! its selector writes the set's content in the frame it bakes the block; a
//! set copy (a block moving from a high to a low set) blits the old content.
//! Exarust bakes every newly held set from the layers instead of copying
//! (the content is a function of the block alone, so only the filtering of
//! the copy's downsample differs) and keeps the per-pixel draw until the
//! set's bake has run, so a block never shows an unwritten set.

use bevy::prelude::*;

use exa_render::terrain_composite::{
    HostedSet, TerrainBake, TerrainBakeParams, TerrainBakeQueue, TerrainCompositeHost,
};

use super::live::TerrainStreaming;
use super::{TerrainBlock, TerrainSplat, composite, terrain_flags};

/// One block's composite draw: the bake in flight and the set it draws.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DrawState {
    /// The bake queued for the block: the set, its colour target and the
    /// queue's ticket.
    pub baking: Option<(u32, AssetId<Image>, u64)>,
    /// The set the block draws from, with the colour target it was baked
    /// into (a rebuilt set has a new one).
    pub drawn: Option<(u32, AssetId<Image>)>,
}

/// One change to a block's composite draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawStep {
    /// The block lost the set it drew: back to its tile's material.
    Restore(usize),
    /// The block lost the set before its bake ran.
    Cancel(usize, u64),
    /// Bake the block into the set it holds.
    Bake(usize, u32),
    /// The block's bake ran: draw from the set.
    Draw(usize, u32),
}

/// This frame's composite draw steps. `held[i]` is the set block `i` holds
/// and `hosted(set)` the colour target of a hosted set; `order` is the
/// selection order the bakes follow and `finished` the tickets the renderer
/// reported. At most `budget` bakes are in flight at once (the original
/// bakes one block a frame).
pub fn draw_steps(
    held: &[Option<u32>],
    hosted: impl Fn(u32) -> Option<AssetId<Image>>,
    states: &[DrawState],
    order: &[usize],
    finished: &[u64],
    budget: usize,
) -> Vec<DrawStep> {
    let mut steps = Vec::new();
    let mut in_flight = 0;
    for (block, state) in states.iter().enumerate() {
        let holding = held.get(block).copied().flatten();
        let current = holding.and_then(|set| hosted(set).map(|colour| (set, colour)));
        if let Some((set, colour, ticket)) = state.baking {
            if current != Some((set, colour)) {
                steps.push(DrawStep::Cancel(block, ticket));
            } else if finished.contains(&ticket) {
                steps.push(DrawStep::Draw(block, set));
            } else {
                in_flight += 1;
            }
        }
        if let Some(drawn) = state.drawn
            && current != Some(drawn)
        {
            steps.push(DrawStep::Restore(block));
        }
    }
    for &block in order {
        if in_flight >= budget {
            break;
        }
        let Some(state) = states.get(block) else {
            continue;
        };
        let Some(set) = held.get(block).copied().flatten() else {
            continue;
        };
        let Some(colour) = hosted(set) else {
            continue;
        };
        let baking = state
            .baking
            .is_some_and(|(s, c, _)| (s, c) == (set, colour));
        if baking || state.drawn == Some((set, colour)) {
            continue;
        }
        steps.push(DrawStep::Bake(block, set));
        in_flight += 1;
    }
    steps
}

/// Where a block's composite sits: its low world corner and side, its tile's
/// centre, and its window on the tile's vertex coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlockFrame {
    /// The block's low x, z corner, metres (world).
    pub low: Vec2,
    /// The block's side, metres.
    pub side: f32,
    /// The tile's centre x, z, metres (world).
    pub tile_centre: Vec2,
    /// The block's low corner on the tile's 0–1 vertex coordinates.
    pub tile_low: Vec2,
    /// The block's span on them (1 / 2^level).
    pub tile_span: f32,
}

impl BlockFrame {
    /// The frame of block (`bx`, `by`) at `level`, from its centre and half
    /// extents (metres, world).
    pub fn new(block: &TerrainBlock, centre: Vec2, half: Vec3) -> Self {
        let side = half.x * 2.0;
        let split = (1u32 << block.level.min(31)) as f32;
        let low = centre - Vec2::new(half.x, half.z);
        let tile_side = side * split;
        let tile_low = Vec2::new(block.bx as f32, block.by as f32) / split;
        Self {
            low,
            side,
            tile_centre: low - tile_low * tile_side + Vec2::splat(tile_side * 0.5),
            tile_low,
            tile_span: 1.0 / split,
        }
    }
}

/// The bake of a block into `targets`, from its tile's material.
pub fn block_bake(
    material: &TerrainSplat,
    frame: &BlockFrame,
    set: u32,
    targets: HostedSet,
) -> Option<TerrainBake> {
    let layers = (material.params.layers as usize).min(super::SPLAT_LAYERS);
    let params = TerrainBakeParams {
        colour: material.params.colour,
        blend: material.params.blend,
        normal: material.params.normal,
        surface: material.params.surface,
        block: Vec4::new(frame.low.x, frame.low.y, frame.side, 0.0),
        tile: Vec4::new(frame.tile_low.x, frame.tile_low.y, frame.tile_span, 0.0),
        layers: layers as u32,
        ..default()
    };
    Some(TerrainBake {
        set,
        targets,
        params,
        layer_sampler: material.layer_sampler.clone()?,
        covers: material.splat.clone()?,
        layers: (0..layers)
            .map(|k| {
                material
                    .layer(k)
                    .map(|[d, s, n]| [d.clone(), s.clone(), n.clone()])
                    .unwrap_or_default()
            })
            .collect(),
    })
}

/// The block's own material: its tile's, reading the baked set at the
/// block's `TexPos`. A set without all four targets has none.
pub fn composite_material(
    tile: &TerrainSplat,
    block: &TerrainBlock,
    frame: &BlockFrame,
    set: &HostedSet,
) -> Option<TerrainSplat> {
    let mut material = tile.clone();
    material.composite_colour = Some(set.colour.clone());
    material.composite_normal = Some(set.normal.clone()?);
    material.composite_gloss = Some(set.gloss.clone()?);
    material.composite_height = Some(set.displacement.clone()?);
    let pos = composite::tex_pos(composite::TILE_LEVEL, block.level, block.bx, block.by);
    material.params.composite_pos = pos.extend(0.0);
    material.params.composite_origin =
        Vec4::new(frame.tile_centre.x, frame.tile_centre.y, 0.0, 0.0);
    material.params.flags |= terrain_flags::COMPOSITE;
    Some(material)
}

/// The blocks' composite draw states and the materials they hold.
#[derive(Resource, Default)]
pub struct CompositeDraws {
    states: Vec<DrawState>,
    /// Per block: its tile's material, once it has drawn its own.
    tile_materials: Vec<Option<Handle<TerrainSplat>>>,
    /// Per block: its own material.
    own: Vec<Option<Handle<TerrainSplat>>>,
}

impl CompositeDraws {
    pub fn states(&self) -> &[DrawState] {
        &self.states
    }

    /// How many blocks draw from a set.
    pub fn drawn(&self) -> usize {
        self.states.iter().filter(|s| s.drawn.is_some()).count()
    }

    fn reset(&mut self, blocks: usize) {
        self.states = vec![DrawState::default(); blocks];
        self.tile_materials = vec![None; blocks];
        self.own = vec![None; blocks];
    }
}

/// Runs the frame's composite draw steps after the selection: queues one
/// bake, swaps baked blocks onto their set and back when they lose it.
#[allow(clippy::too_many_arguments)]
pub fn draw_terrain_composites(
    mut commands: Commands,
    streaming: Option<Res<TerrainStreaming>>,
    host: Option<Res<TerrainCompositeHost>>,
    queue: Option<ResMut<TerrainBakeQueue>>,
    mut draws: ResMut<CompositeDraws>,
    mut materials: ResMut<Assets<TerrainSplat>>,
    blocks: Query<(&TerrainBlock, &MeshMaterial3d<TerrainSplat>)>,
) {
    let (Some(streaming), Some(host), Some(mut queue)) = (streaming, host, queue) else {
        return;
    };
    if !host.enabled() {
        return;
    }
    let live = streaming.blocks();
    if streaming.is_added() || draws.states.len() != live.len() {
        draws.reset(live.len());
    }
    let finished = queue.finished();
    let drawn_before = draws.drawn();
    let held: Vec<Option<u32>> = streaming
        .selector()
        .blocks()
        .iter()
        .map(|block| block.set())
        .collect();
    let hosted = |set: u32| host.set(set).map(|hosted| hosted.colour.id());
    let steps = draw_steps(
        &held,
        hosted,
        &draws.states,
        streaming.selector().selected(),
        &finished,
        1,
    );
    for step in steps {
        match step {
            DrawStep::Cancel(index, ticket) => {
                queue.cancel(ticket);
                draws.states[index].baking = None;
            }
            DrawStep::Restore(index) => {
                draws.states[index].drawn = None;
                draws.own[index] = None;
                if let Some(tile) = draws.tile_materials[index].clone() {
                    commands
                        .entity(live[index].entity)
                        .insert(MeshMaterial3d(tile));
                }
            }
            DrawStep::Bake(index, set) => {
                let (Some(targets), Ok((block, material))) =
                    (host.set(set), blocks.get(live[index].entity))
                else {
                    continue;
                };
                // The tile's material: the block's own while it draws one.
                let tile = draws.tile_materials[index]
                    .clone()
                    .unwrap_or_else(|| material.0.clone());
                let Some(source) = materials.get(&tile) else {
                    continue;
                };
                let frame = BlockFrame::new(block, live[index].centre, live[index].half);
                if let Some(bake) = block_bake(source, &frame, set, targets.clone()) {
                    let ticket = queue.push(bake);
                    draws.states[index].baking = Some((set, targets.colour.id(), ticket));
                }
            }
            DrawStep::Draw(index, set) => {
                let colour = draws.states[index].baking.map(|(_, colour, _)| colour);
                draws.states[index].baking = None;
                let (Some(targets), Some(colour), Ok((block, material))) =
                    (host.set(set), colour, blocks.get(live[index].entity))
                else {
                    continue;
                };
                let tile = draws.tile_materials[index]
                    .clone()
                    .unwrap_or_else(|| material.0.clone());
                let frame = BlockFrame::new(block, live[index].centre, live[index].half);
                let Some(own) = materials
                    .get(&tile)
                    .and_then(|source| composite_material(source, block, &frame, targets))
                else {
                    continue;
                };
                let own = materials.add(own);
                commands
                    .entity(live[index].entity)
                    .insert(MeshMaterial3d(own.clone()));
                draws.tile_materials[index] = Some(tile);
                draws.own[index] = Some(own);
                draws.states[index].drawn = Some((set, colour));
            }
        }
    }
    let drawn = draws.drawn();
    if drawn != drawn_before {
        if drawn_before == 0 {
            info!("terrain: the first block draws from its baked composite set");
        }
        debug!(
            "terrain: {drawn} blocks draw from baked composite sets ({} sets hosted, {} bakes queued)",
            host.len(),
            queue.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(n: u128) -> AssetId<Image> {
        AssetId::Uuid {
            uuid: bevy::asset::uuid::Uuid::from_u128(n),
        }
    }

    #[test]
    fn one_held_set_is_baked_at_a_time_in_selection_order() {
        let held = [Some(4), None, Some(9)];
        let hosted = |set: u32| Some(image(u128::from(set)));
        let states = [DrawState::default(); 3];
        // Block 2 comes first in the selection.
        let steps = draw_steps(&held, hosted, &states, &[2, 0], &[], 1);
        assert_eq!(steps, [DrawStep::Bake(2, 9)]);
        // While it bakes, nothing else is queued.
        let mut states = states;
        states[2].baking = Some((9, image(9), 7));
        assert!(draw_steps(&held, hosted, &states, &[2, 0], &[], 1).is_empty());
        // Its ticket done, it draws and the next block bakes.
        let steps = draw_steps(&held, hosted, &states, &[2, 0], &[7], 1);
        assert_eq!(steps, [DrawStep::Draw(2, 9), DrawStep::Bake(0, 4)]);
    }

    #[test]
    fn a_set_not_yet_hosted_waits() {
        let held = [Some(4)];
        let steps = draw_steps(&held, |_| None, &[DrawState::default()], &[0], &[], 1);
        assert!(steps.is_empty());
    }

    #[test]
    fn a_lost_or_rebuilt_set_goes_back_to_the_tile() {
        let hosted = |set: u32| Some(image(u128::from(set)));
        let drawn = DrawState {
            baking: None,
            drawn: Some((4, image(4))),
        };
        // Still held: nothing to do.
        assert!(draw_steps(&[Some(4)], hosted, &[drawn], &[0], &[], 1).is_empty());
        // Lost: back to the tile's material.
        assert_eq!(
            draw_steps(&[None], hosted, &[drawn], &[0], &[], 1),
            [DrawStep::Restore(0)]
        );
        // The set's targets rebuilt (a new colour image): restore and bake
        // again.
        let rebuilt = |_: u32| Some(image(99));
        assert_eq!(
            draw_steps(&[Some(4)], rebuilt, &[drawn], &[0], &[], 1),
            [DrawStep::Restore(0), DrawStep::Bake(0, 4)]
        );
        // A bake whose set went away is cancelled.
        let baking = DrawState {
            baking: Some((4, image(4), 3)),
            drawn: None,
        };
        assert_eq!(
            draw_steps(&[Some(5)], hosted, &[baking], &[0], &[], 1),
            [DrawStep::Cancel(0, 3), DrawStep::Bake(0, 5)]
        );
    }

    #[test]
    fn a_block_frame_finds_its_tile_and_window() {
        // Block (2, 5) of a 48 m tile centred at (100, −40): 6 m blocks, its
        // centre 3 m in from (2·6 − 24, 5·6 − 24).
        let block = TerrainBlock {
            bx: 2,
            by: 5,
            level: 3,
            room: None,
        };
        let centre = Vec2::new(100.0 - 12.0 + 3.0, -40.0 + 6.0 + 3.0);
        let frame = BlockFrame::new(&block, centre, Vec3::new(3.0, 0.1, 3.0));
        assert_eq!(frame.side, 6.0);
        assert_eq!(frame.low, Vec2::new(88.0, -34.0));
        assert_eq!(frame.tile_centre, Vec2::new(100.0, -40.0));
        assert_eq!(frame.tile_low, Vec2::new(0.25, 0.625));
        assert_eq!(frame.tile_span, 0.125);
    }

    #[test]
    fn the_composite_coordinate_spans_the_block() {
        // The draw's coordinate: tile-local centimetres × TexPos.z + xy is 0
        // at the block's low corner and 1 at its high corner.
        let block = TerrainBlock {
            bx: 2,
            by: 5,
            level: 3,
            room: None,
        };
        let frame = BlockFrame::new(&block, Vec2::new(91.0, -31.0), Vec3::new(3.0, 0.1, 3.0));
        let pos = composite::tex_pos(composite::TILE_LEVEL, block.level, block.bx, block.by);
        let uv = |world: Vec2| (world - frame.tile_centre) * 100.0 * pos.z + pos.truncate();
        assert!((uv(frame.low) - Vec2::ZERO).length() < 1e-4);
        assert!((uv(frame.low + Vec2::splat(6.0)) - Vec2::ONE).length() < 1e-4);
    }

    #[test]
    fn the_bake_carries_the_tiles_layers_and_the_blocks_window() {
        let mut images = Assets::<Image>::default();
        let mut tile = TerrainSplat {
            layer_sampler: Some(images.add(Image::default())),
            splat: Some(images.add(Image::default())),
            ..default()
        };
        let diffuse = images.add(Image::default());
        tile.params.layers = 2;
        tile.params.colour[1] = Vec4::new(0.5, 0.25, 1.0, 0.8);
        tile.diffuse1 = Some(diffuse.clone());
        let block = TerrainBlock {
            bx: 2,
            by: 5,
            level: 3,
            room: None,
        };
        let frame = BlockFrame::new(&block, Vec2::new(91.0, -31.0), Vec3::new(3.0, 0.1, 3.0));
        let targets = HostedSet {
            level: 5,
            colour: images.add(Image::default()),
            normal: Some(images.add(Image::default())),
            gloss: Some(images.add(Image::default())),
            displacement: Some(images.add(Image::default())),
        };
        let bake = block_bake(&tile, &frame, 3, targets.clone()).unwrap();
        assert_eq!(bake.layers.len(), 2);
        assert_eq!(bake.layers[1][0], Some(diffuse));
        assert_eq!(bake.params.colour[1], tile.params.colour[1]);
        assert_eq!(bake.params.layers, 2);
        assert_eq!(bake.params.block, Vec4::new(88.0, -34.0, 6.0, 0.0));
        assert_eq!(bake.params.tile, Vec4::new(0.25, 0.625, 0.125, 0.0));
        // The block's own material reads the set at its TexPos.
        let own = composite_material(&tile, &block, &frame, &targets).unwrap();
        assert_eq!(own.composite_colour, Some(targets.colour.clone()));
        assert_eq!(
            own.params.composite_pos,
            Vec4::new(2.0, -1.0, 8.0 / 4800.0, 0.0)
        );
        assert_eq!(
            own.params.composite_origin,
            Vec4::new(100.0, -40.0, 0.0, 0.0)
        );
        assert_ne!(own.params.flags & terrain_flags::COMPOSITE, 0);
        // A tile without its covers cannot bake; a set without a target is
        // not drawn.
        let bare = TerrainSplat::default();
        assert!(block_bake(&bare, &frame, 3, targets.clone()).is_none());
        let partial = HostedSet {
            gloss: None,
            ..targets
        };
        assert!(composite_material(&tile, &block, &frame, &partial).is_none());
    }
}
