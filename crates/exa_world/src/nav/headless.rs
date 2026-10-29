//! A level's grid built without a window, for `exa-tool nav` and the corpus
//! tests: the level is spawned into a bare world exactly as the game spawns
//! it, its nodes' world transforms are worked out from their parents, and
//! the same inputs are collected.

use std::collections::HashMap;

use bevy::ecs::world::CommandQueue;
use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_assets::{GameData, ModelCache, ModelLoader};

use super::view::{Ground, MovingMesh, PendingNav, floor_mesh};
use super::{NavGrid, NavSource};
use crate::gi_volume::{GiFloor, GiPoints};

/// What a headless build produced.
pub struct HeadlessLevel {
    pub source: NavSource,
    pub info: crate::LevelInfo,
}

impl HeadlessLevel {
    pub fn build(&self) -> NavGrid {
        self.source.build()
    }
}

/// Spawns `name` and collects its grid's inputs; `None` when the level is
/// missing or does not parse.
pub fn level_source(game: &GameData, name: &str) -> Option<HeadlessLevel> {
    let mut world = World::new();
    let mut queue = CommandQueue::default();
    let mut cache = ModelCache::default();
    let mut meshes = Assets::<Mesh>::default();
    let mut materials = Assets::<exa_render::material::ExaMaterial>::default();
    let mut images = Assets::<Image>::default();
    let mut loader = ModelLoader {
        game,
        cache: &mut cache,
        meshes: &mut meshes,
        materials: &mut materials,
        images: &mut images,
        bc_supported: true,
    };
    let (root, info) = {
        let mut commands = Commands::new(&mut queue, &world);
        crate::spawn_level(&mut commands, &mut loader, name)?
    };
    queue.apply(&mut world);
    let pending = *world.get_resource::<PendingNav>()?;
    let mut source = NavSource {
        width: pending.width,
        depth: pending.depth,
        base: pending.base,
        ..Default::default()
    };
    let mut globals: HashMap<Entity, Affine3A> = HashMap::new();
    let mut stack = vec![root];
    while let Some(e) = stack.pop() {
        let entity = world.entity(e);
        let local = entity
            .get::<Transform>()
            .map_or(Affine3A::IDENTITY, Transform::compute_affine);
        let global = match entity.get::<ChildOf>() {
            Some(parent) => globals.get(&parent.parent()).copied().unwrap_or_default() * local,
            None => local,
        };
        globals.insert(e, global);
        if let Some(gi) = entity.get::<GiPoints>() {
            let moving = entity.get::<MovingMesh>();
            if moving.is_none_or(|m| m.stamped) {
                source.meshes.push((gi.0.clone(), global));
            }
            if moving.is_some() {
                source.movers.push((gi.0.clone(), global));
            }
        }
        if entity.contains::<GiFloor>()
            && let Some(mesh) = entity.get::<Mesh3d>()
            && let Some(floor) = floor_mesh(meshes.get(&mesh.0), global)
        {
            source.floors.push(floor);
        }
        if let Some(g) = entity.get::<Ground>() {
            source.ground.push(g.0.clone());
        }
        if let Some(children) = entity.get::<Children>() {
            stack.extend(children.iter());
        }
    }
    Some(HeadlessLevel { source, info })
}
