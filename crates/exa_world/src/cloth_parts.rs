//! The body parts' cell index the cloth stage reads: every part that can
//! meet a simulated object, linked in the 6 m cell its mesh box's world
//! centre falls in, as the grid sector's body grid links it.
//!
//! The original re-buckets every body each tick. A body whose `Ragdoll`
//! was not changed since the last tick (a deep sleeper: the body stage
//! leaves it untouched, as its drawing and its prop stand-ins already rely
//! on) has the same parts in the same places, so its cells are the same;
//! the index re-buckets only the changed bodies and keeps the rest, which
//! gives the same cells as a full re-bucket. The stage then reads only the
//! cells around its cloths, each cell's parts in the order a full re-bucket
//! would have linked them (the bodies' walk order this tick, then each
//! body's part order).

use bevy::ecs::entity::EntityHashMap;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use exa_physics::body::Ragdoll;
use exa_physics::sector;

/// One body's entry in the index.
#[derive(Debug, Default)]
struct BodyCells {
    /// The body's place in this tick's walk of the bodies.
    rank: u32,
    /// The refresh that last walked it.
    seen: u32,
    /// Each linked part: its index in the body and its cell.
    linked: Vec<(usize, (i32, i32))>,
    /// Every part's entity (linked or not), for the owner lookup.
    entities: Vec<Entity>,
}

/// A linked part in a cell: its body, the body's walk rank and the part's
/// index in the body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LinkedPart {
    pub rank: u32,
    pub part: usize,
    pub body: Entity,
}

/// The incremental cell index of the bodies' parts (see the module).
#[derive(Debug, Default)]
pub struct PartCells {
    bodies: EntityHashMap<BodyCells>,
    /// Every part's body, by the part's entity.
    owners: EntityHashMap<Entity>,
    /// Each cell's linked parts as (body, part index), unordered.
    cells: HashMap<(i32, i32), Vec<(Entity, usize)>>,
    refresh: u32,
    /// Bodies re-bucketed by the last refresh.
    pub rebucketed: usize,
}

impl PartCells {
    /// An empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Forgets every body (nothing reads the index while no cloth is
    /// simulated; the next refresh buckets every body afresh).
    pub fn clear(&mut self) {
        self.bodies.clear();
        self.owners.clear();
        self.cells.clear();
        self.rebucketed = 0;
    }

    /// Walks this tick's bodies in their order, each with whether it was
    /// changed since the last refresh: records its rank, re-buckets a
    /// changed or new one, and drops the bodies that are gone.
    pub fn refresh<'a>(
        &mut self,
        bodies: impl Iterator<Item = (Entity, bool, &'a Ragdoll)>,
        origin: (i32, i32),
    ) {
        self.refresh = self.refresh.wrapping_add(1);
        let refresh = self.refresh;
        let mut walked = 0usize;
        self.rebucketed = 0;
        for (rank, (entity, changed, body)) in bodies.enumerate() {
            walked += 1;
            let rank = rank as u32;
            if !changed && let Some(entry) = self.bodies.get_mut(&entity) {
                entry.rank = rank;
                entry.seen = refresh;
                continue;
            }
            let mut entry = self.bodies.remove(&entity).unwrap_or_default();
            self.unlink(entity, &mut entry);
            entry.rank = rank;
            entry.seen = refresh;
            for (index, part) in body.parts.iter().enumerate() {
                self.owners.insert(part.entity, entity);
                entry.entities.push(part.entity);
                if part.pairing.out || part.spheres.is_empty() {
                    continue;
                }
                let now = part.transform();
                let box_centre = part.pairing.mesh.world_centre(&now) * 100.0;
                let cell = sector::cell_of(box_centre, origin);
                entry.linked.push((index, cell));
                self.cells.entry(cell).or_default().push((entity, index));
            }
            self.bodies.insert(entity, entry);
            self.rebucketed += 1;
        }
        if self.bodies.len() > walked {
            let gone: Vec<Entity> = self
                .bodies
                .iter()
                .filter(|(_, entry)| entry.seen != refresh)
                .map(|(&entity, _)| entity)
                .collect();
            for entity in gone {
                if let Some(mut entry) = self.bodies.remove(&entity) {
                    self.unlink(entity, &mut entry);
                }
            }
        }
    }

    /// Takes a body's parts out of the cells and the owner lookup, leaving
    /// its entry's lists empty for reuse.
    fn unlink(&mut self, entity: Entity, entry: &mut BodyCells) {
        for (index, cell) in entry.linked.drain(..) {
            if let Some(list) = self.cells.get_mut(&cell)
                && let Some(at) = list.iter().position(|&x| x == (entity, index))
            {
                list.swap_remove(at);
            }
        }
        for part in entry.entities.drain(..) {
            if self.owners.get(&part) == Some(&entity) {
                self.owners.remove(&part);
            }
        }
    }

    /// The body a part entity belongs to, whether its part is linked or not.
    pub fn owner_of(&self, part: Entity) -> Option<Entity> {
        self.owners.get(&part).copied()
    }

    /// The parts linked in `cell`, in the order a full re-bucket this tick
    /// links them: by the bodies' walk rank, then by part index.
    pub fn sorted_in(&self, cell: (i32, i32), out: &mut Vec<LinkedPart>) {
        out.clear();
        let Some(list) = self.cells.get(&cell) else {
            return;
        };
        out.extend(list.iter().filter_map(|&(body, part)| {
            self.bodies.get(&body).map(|entry| LinkedPart {
                rank: entry.rank,
                part,
                body,
            })
        }));
        out.sort_unstable();
    }

    /// The number of linked parts in the index.
    pub fn linked(&self) -> usize {
        self.cells.values().map(Vec::len).sum()
    }
}

/// The distinct cells of the nine around each of `centres`, sorted.
pub fn cells_around(centres: impl Iterator<Item = (i32, i32)>, out: &mut Vec<(i32, i32)>) {
    out.clear();
    for centre in centres {
        for (dx, dz) in sector::NEAR_OFFSETS {
            out.push((centre.0.wrapping_add(dx), centre.1.wrapping_add(dz)));
        }
    }
    out.sort_unstable();
    out.dedup();
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Affine3A;
    use exa_physics::body::PartSetup;

    /// A body of `parts` parts in a row along x from `at`, each part's
    /// entity from `world`.
    fn body(world: &mut World, at: Vec3, parts: usize) -> Ragdoll {
        let setups: Vec<PartSetup> = (0..parts)
            .map(|part| PartSetup {
                bone: part,
                entity: world.spawn_empty().id(),
                parent: None,
                world: Affine3A::from_translation(at + Vec3::new(part as f32 * 0.2, 0.0, 0.0)),
                centre: Vec3::ZERO,
                half: Vec3::splat(0.05),
                origin: Vec3::ZERO,
                links: vec![],
                foot: None,
                pins: Vec::new(),
                spheres: Some(vec![(Vec3::ZERO, 0.06)]),
            })
            .collect();
        Ragdoll::new(&setups)
    }

    /// The bodies' walk: each with its id and whether it changed.
    fn walk<'a>(
        ids: &[Entity],
        bodies: &'a [Ragdoll],
        changed: [bool; 3],
    ) -> Vec<(Entity, bool, &'a Ragdoll)> {
        ids.iter()
            .zip(changed)
            .zip(bodies)
            .map(|((&e, c), b)| (e, c, b))
            .collect()
    }

    /// Every cell's sorted parts, for comparing two indexes.
    fn snapshot(index: &PartCells, cells: &[(i32, i32)]) -> Vec<Vec<LinkedPart>> {
        let mut out = Vec::new();
        cells
            .iter()
            .map(|&cell| {
                index.sorted_in(cell, &mut out);
                out.clone()
            })
            .collect()
    }

    /// The index after an unchanged tick and after a changed body moved
    /// holds what a fresh full re-bucket holds, in the same order.
    #[test]
    fn the_incremental_index_matches_a_full_rebucket() {
        let mut world = World::new();
        let ids: Vec<Entity> = (0..3).map(|_| world.spawn_empty().id()).collect();
        let mut bodies = vec![
            body(&mut world, Vec3::new(1.0, 0.0, 1.0), 3),
            body(&mut world, Vec3::new(2.0, 0.0, 1.5), 2),
            body(&mut world, Vec3::new(13.0, 0.0, 1.0), 2),
        ];
        let cells: Vec<(i32, i32)> = (-1..4).flat_map(|x| (-1..2).map(move |z| (x, z))).collect();
        let mut index = PartCells::new();
        index.refresh(walk(&ids, &bodies, [true; 3]).into_iter(), (0, 0));
        assert_eq!(index.rebucketed, 3);
        assert_eq!(index.linked(), 7);
        // An unchanged tick re-buckets nothing.
        index.refresh(walk(&ids, &bodies, [false; 3]).into_iter(), (0, 0));
        assert_eq!(index.rebucketed, 0);
        // The first body moves 6 m on (changed); the others stay.
        bodies[0].translate(Vec3::new(6.0, 0.0, 0.0));
        index.refresh(
            walk(&ids, &bodies, [true, false, false]).into_iter(),
            (0, 0),
        );
        assert_eq!(index.rebucketed, 1);
        let mut fresh = PartCells::new();
        fresh.refresh(walk(&ids, &bodies, [true; 3]).into_iter(), (0, 0));
        assert_eq!(snapshot(&index, &cells), snapshot(&fresh, &cells));
        // A cell's parts come by walk rank, then part index.
        let mut first = Vec::new();
        index.sorted_in((0, 0), &mut first);
        assert!(first.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(first.first().map(|p| p.body), Some(ids[1]));
        // A body that is gone leaves the index and the owner lookup.
        let gone = bodies[2].parts[0].entity;
        assert_eq!(index.owner_of(gone), Some(ids[2]));
        index.refresh(walk(&ids, &bodies, [false; 3]).into_iter().take(2), (0, 0));
        assert_eq!(index.owner_of(gone), None);
        assert_eq!(index.linked(), 5);
    }

    /// The nine cells around each centre, once each.
    #[test]
    fn cells_around_two_neighbours_are_twelve() {
        let mut out = Vec::new();
        cells_around([(0, 0), (1, 0)].into_iter(), &mut out);
        assert_eq!(out.len(), 12);
        assert!(out.windows(2).all(|w| w[0] < w[1]));
    }
}
