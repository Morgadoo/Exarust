//! The thaumaturgy casts' scene picks: the navigation unit's ground pick
//! and nearest-mesh pick ([`crate::nav_pick`]) asked from the cursor's ray
//! where the original's casts ask them (`docs/THAUMATURGY.md`, Cast picks).
//!
//! - **Ground pick** ([`CastPicker::ground`]): the throw from the cursor
//!   (its accept test takes a node with an object: a prop's or a
//!   character's), the barricade's placement (it takes a node without one:
//!   the level) and the long door's and the summoning's destination (it
//!   takes every node). The accept test only grows the node's sphere (5 cm
//!   taken, 0.1 cm refused); the nearest node met is the pick whatever it
//!   is. The barricade, the long door and the summoning take the hit as it
//!   stands on a floor mesh (its scene node's floor bit) and 15 cm back
//!   along the ray otherwise ([`CastHit::place`]).
//! - **Nearest-mesh pick** ([`CastPicker::mesh_point`]): the barricade's
//!   cast asks it before the front end; the segment from the eye to its
//!   point (the ground plane's point when it meets nothing) meeting a
//!   running barricade's node bounds (sphere, then box) ends that barricade
//!   ([`segment_meets_wall`]).
//!
//! The nodes are Exarust's: the level is one node answered by the physics
//! world's fixed colliders (pick-only meshes included, the props' stand-ins
//! left out), the props one node answered by their stand-ins, and each
//! character one node answered by its limbs' meshes (or its collision
//! spheres) through the hand's limb ray ([`crate::hand::character_hit`]).
//! Each node's bounds hold the whole pick segment
//! ([`Candidate::enclosing`]), so only the nearest-first rule picks.
//! [inf: the original walks its sectors' geometry nodes and asks each
//! mesh's own column or ray callback; Exarust's nodes are these stand-ins.]
//!
//! Evidence: the throw's pick from the cursor, the barricade's cast and
//! placement, the long door's and the summoning's front ends and the three
//! accept callbacks, read from `Exanima.exe`.

use std::cell::Cell;
use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use exa_combat::Condition;
use exa_physics::body::Ragdoll;
use exa_physics::collision_share::FramePhysics;
use exa_physics::intersect::{Obb, segment_obb_hit, segment_sphere};
use exa_physics::prop::{PropBody, StandIns};
use exa_physics::{ColliderHandle, Physics, StaticColliders};

use super::barricade::Barricade;
use super::kinesis_live::KinesisCursor;
use crate::nav_pick::{
    Candidate, GROUND_FAR, Ray, RayMesh, floor_distance, ground_hit, nearest_hit,
};
use crate::player::ChaseCamera;

const CM: f32 = 100.0;
/// How far back along the ray a hit off a floor mesh is taken (cm).
pub const BACK_OFF: f32 = 15.0;
/// The mesh pick's length when the camera has no far plane (m).
const DEFAULT_FAR: f32 = 1000.0;

/// What holds a cast pick's hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    /// The level's meshes; `floor` when the mesh is a floor (its scene
    /// node's second word carries the floor bit).
    Level { floor: bool },
    /// A body: a prop or a character.
    Body(Entity),
}

/// A cast pick's hit: the point (m), the ray's unit direction, how far
/// along it, and its node's owner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CastHit {
    pub point: Vec3,
    pub direction: Vec3,
    pub distance: f32,
    pub owner: Owner,
}

impl CastHit {
    /// Whether the hit's mesh keeps the point as it stands.
    pub fn exact(&self) -> bool {
        self.owner == Owner::Level { floor: true }
    }

    /// Where a cast that takes the ground puts its point (cm): the hit on a
    /// floor mesh, else [`BACK_OFF`] back along the ray.
    pub fn place(&self) -> Vec3 {
        let point = self.point * CM;
        if self.exact() {
            point
        } else {
            point + self.direction * -BACK_OFF
        }
    }
}

/// The level node's mesh: the fixed colliders but the props' stand-ins.
/// It records the collider its last answer met.
struct LevelMesh<'a> {
    physics: &'a Physics,
    stand_ins: &'a HashMap<ColliderHandle, Entity>,
    met: Cell<Option<ColliderHandle>>,
}

impl RayMesh for LevelMesh<'_> {
    fn nearest_hit(&self, ray: Ray, within: f32) -> Option<(usize, f32)> {
        let keep = |h: ColliderHandle| !self.stand_ins.contains_key(&h);
        let (handle, distance) =
            self.physics
                .fixed_pick_where(ray.origin, ray.direction, within, &keep)?;
        self.met.set(Some(handle));
        Some((0, distance))
    }
}

/// The props' node: their stand-ins. It records the body its last answer
/// met.
struct PropsMesh<'a> {
    physics: &'a Physics,
    stand_ins: &'a HashMap<ColliderHandle, Entity>,
    met: Cell<Option<Entity>>,
}

impl RayMesh for PropsMesh<'_> {
    fn nearest_hit(&self, ray: Ray, within: f32) -> Option<(usize, f32)> {
        let keep = |h: ColliderHandle| self.stand_ins.contains_key(&h);
        let (handle, distance) =
            self.physics
                .fixed_pick_where(ray.origin, ray.direction, within, &keep)?;
        self.met.set(self.stand_ins.get(&handle).copied());
        Some((0, distance))
    }
}

/// A character's node: its limbs.
struct CharacterMesh<'a> {
    body: Entity,
    ragdoll: &'a Ragdoll,
    condition: Condition,
}

impl RayMesh for CharacterMesh<'_> {
    fn nearest_hit(&self, ray: Ray, within: f32) -> Option<(usize, f32)> {
        let one = std::iter::once((self.body, self.ragdoll, self.condition));
        crate::hand::character_hit(ray.origin, ray.direction, within, one).map(|(t, _)| (0, t))
    }
}

/// The scene the cast picks read.
#[derive(SystemParam)]
pub struct CastPicker<'w, 's> {
    cursor: Res<'w, KinesisCursor>,
    physics: Option<Res<'w, FramePhysics>>,
    stand_ins: Option<Res<'w, StandIns>>,
    statics: Option<Res<'w, StaticColliders>>,
    bodies: Query<
        'w,
        's,
        (
            Entity,
            &'static Ragdoll,
            Option<&'static Condition>,
            Has<PropBody>,
        ),
    >,
    cameras: Query<'w, 's, (&'static GlobalTransform, &'static Projection), With<ChaseCamera>>,
}

/// One pick's nodes: the level first, then the props, then each character.
struct Nodes<'a> {
    level: LevelMesh<'a>,
    props: PropsMesh<'a>,
    characters: Vec<CharacterMesh<'a>>,
}

impl Nodes<'_> {
    fn candidates(&self, origin: Vec3, segment: Vec3) -> Vec<Candidate<'_>> {
        let mut out = vec![
            Candidate::enclosing(origin, segment, &self.level),
            Candidate::enclosing(origin, segment, &self.props),
        ];
        out.extend(
            self.characters
                .iter()
                .map(|c| Candidate::enclosing(origin, segment, c)),
        );
        out
    }

    /// The owner of candidate `node`: 0 the level, 1 the props, then the
    /// characters in order.
    fn owner(&self, node: usize, statics: Option<&StaticColliders>) -> Option<Owner> {
        match node {
            0 => Some(Owner::Level {
                floor: self
                    .level
                    .met
                    .get()
                    .is_some_and(|h| statics.is_some_and(|s| s.is_floor(h))),
            }),
            1 => self.props.met.get().map(Owner::Body),
            n => self.characters.get(n - 2).map(|c| Owner::Body(c.body)),
        }
    }
}

impl CastPicker<'_, '_> {
    /// The cursor's ray (m, unit direction), when the pointer is over the
    /// view.
    pub fn ray(&self) -> Option<Ray> {
        let (origin, direction) = self.cursor.0?;
        Some(Ray::new(origin, direction).unit())
    }

    /// The camera's eye (m).
    pub fn eye(&self) -> Option<Vec3> {
        self.cameras.iter().next().map(|(at, _)| at.translation())
    }

    fn with_nodes<T>(
        &self,
        run: impl FnOnce(&Nodes<'_>, Option<&StaticColliders>) -> T,
    ) -> Option<T> {
        let physics = self.physics.as_deref()?;
        let owners: HashMap<ColliderHandle, Entity> = self
            .stand_ins
            .as_deref()
            .map(|s| {
                s.0.iter()
                    .flat_map(|(&body, list)| list.iter().map(move |&(h, _)| (h, body)))
                    .collect()
            })
            .unwrap_or_default();
        let characters = self
            .bodies
            .iter()
            .filter(|(.., prop)| !prop)
            .filter_map(|(body, ragdoll, condition, _)| {
                Some(CharacterMesh {
                    body,
                    ragdoll,
                    condition: *condition?,
                })
            })
            .collect();
        let nodes = Nodes {
            level: LevelMesh {
                physics,
                stand_ins: &owners,
                met: Cell::new(None),
            },
            props: PropsMesh {
                physics,
                stand_ins: &owners,
                met: Cell::new(None),
            },
            characters,
        };
        Some(run(&nodes, self.statics.as_deref()))
    }

    /// The ground pick from the cursor: `accept` answers whether a node
    /// with an object (`true`: a prop or a character) or without one (the
    /// level) grows its sphere 5 cm rather than 0.1 cm. `None` when the
    /// cursor is off the view or nothing is met within 40 m.
    pub fn ground(&self, accept: impl Fn(bool) -> bool) -> Option<CastHit> {
        let ray = self.ray()?;
        self.with_nodes(|nodes, statics| {
            let candidates = nodes.candidates(ray.origin, ray.direction * GROUND_FAR);
            let (hit, _) = ground_hit(ray, u32::MAX, |index, _| accept(index != 0), &candidates);
            let hit = hit?;
            Some(CastHit {
                point: hit.point,
                direction: ray.direction,
                distance: hit.distance,
                owner: nodes.owner(hit.node, statics)?,
            })
        })
        .flatten()
    }

    /// The nearest-mesh pick from the cursor: the point it meets (m), or
    /// the ground plane's point along the ray when it meets nothing, as
    /// the original's pick records. The segment's length is the camera's
    /// near-to-far distance.
    pub fn mesh_point(&self) -> Option<Vec3> {
        let ray = self.ray()?;
        let length = self
            .cameras
            .iter()
            .next()
            .and_then(|(_, projection)| match projection {
                Projection::Perspective(p) => Some(p.far - p.near),
                _ => None,
            })
            .unwrap_or(DEFAULT_FAR);
        self.with_nodes(|nodes, _| {
            let candidates = nodes.candidates(ray.origin, ray.direction * length);
            match nearest_hit(ray, length, u32::MAX, &candidates) {
                Some(hit) => hit.point,
                None => ray.at(floor_distance(ray)),
            }
        })
    }
}

/// The node bounds of a standing barricade (m): its box about the node's
/// place, along the wall's frame, and the box's corner as its sphere.
pub fn wall_bounds(wall: &Barricade) -> Obb {
    let frame = wall.frame;
    Obb::new(
        wall.node_at / CM,
        wall.node_half / CM,
        [frame.x, frame.y, frame.z],
    )
}

/// Whether the segment from `eye` to `point` (m) meets a barricade's node
/// bounds: its sphere first, then its box, as the barricade's cast tests
/// the nearest-mesh pick's segment.
pub fn segment_meets_wall(eye: Vec3, point: Vec3, wall: &Barricade) -> bool {
    let bounds = wall_bounds(wall);
    let segment = point - eye;
    let radius = bounds.half_extents.length();
    segment_sphere(eye, segment, bounds.centre, radius) && segment_obb_hit(eye, segment, &bounds)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(owner: Owner) -> CastHit {
        CastHit {
            point: Vec3::new(1.0, 0.0, 2.0),
            direction: Vec3::new(0.0, -0.6, 0.8),
            distance: 5.0,
            owner,
        }
    }

    #[test]
    fn a_floor_keeps_the_hit_and_anything_else_backs_off_along_the_ray() {
        let floor = hit(Owner::Level { floor: true });
        assert!(floor.exact());
        assert_eq!(floor.place(), Vec3::new(100.0, 0.0, 200.0));
        let wall = hit(Owner::Level { floor: false });
        assert!(!wall.exact());
        let back = wall.place();
        assert!((back - Vec3::new(100.0, 9.0, 188.0)).length() < 1e-4);
        // A body's mesh is not a floor either.
        assert!(!hit(Owner::Body(Entity::PLACEHOLDER)).exact());
    }

    #[test]
    fn the_eye_segment_meets_a_standing_wall_and_misses_beside_it() {
        use super::super::barricade::WallFrame;
        let frame = WallFrame::facing(Vec3::new(0.0, 100.0, 500.0), Vec3::Z);
        let mut wall = Barricade::raise(
            frame,
            0.25,
            0,
            |_, _, _| None,
            None,
            &mut crate::arena::Mt19937::new(1),
        );
        wall.node_at = Vec3::new(0.0, 100.0, 500.0);
        wall.node_half = Vec3::new(300.0, 100.0, 5.0);
        let eye = Vec3::new(0.0, 2.0, 0.0);
        // Through the wall to the ground behind it.
        assert!(segment_meets_wall(eye, Vec3::new(0.0, 0.0, 8.0), &wall));
        // Short of it.
        assert!(!segment_meets_wall(eye, Vec3::new(0.0, 0.0, 3.0), &wall));
        // Past its end.
        assert!(!segment_meets_wall(eye, Vec3::new(9.0, 0.0, 8.0), &wall));
    }
}
