//! The camera's cut-away fade on the spawned scene: the meshes a model
//! marks as cut-away candidates (roofs, awnings, overhangs) fade out while
//! they stand between the camera and the player. This module holds the
//! per-node record the spawners attach ([`CutAway`]) and the two mesh
//! callbacks the detector asks after a node's bounding sphere is met: the
//! camera segment's sphere cast ([`sphere_cast`]) and the overhead probe's
//! ray ([`ray_cast`]). The detector's walk and the fade step are the scene
//! unit's rules ([`crate::scene_nodes::Scene::detect_cut_away`],
//! [`crate::scene_nodes::fade_step`]); `exa_game::cut_away_live` drives
//! them each frame and the renderer draws the result. Rules:
//! `docs/RENDERING.md` (Cut-away fade).

// The callbacks' compares keep the original's NaN outcomes.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use std::sync::Arc;

use bevy::prelude::*;
use exa_assets::{CutAwayMesh, Model};
use exa_formats::rfc::Node;

/// The radius of the sphere the camera segment's mesh callback sweeps (m;
/// the original's 50 cm).
pub const CAST_RADIUS: f32 = 0.5;

/// The smallest determinant (or approach) the mesh callbacks take as
/// non-zero: the engine's single-precision epsilon constant.
pub const CALLBACK_EPSILON: f32 = f32::from_bits(0x33ff_ffd7);

/// The fade target of a mesh with cut-away word `word`: 1 (all the way
/// out) with [`Node::CUT_AWAY_FULL`], else ½, as the mesh loader sets it.
pub fn fade_target(word: u32) -> f32 {
    if word & Node::CUT_AWAY_FULL != 0 {
        1.0
    } else {
        0.5
    }
}

/// A spawned node's cut-away record and live state. Candidates carry their
/// kind in [`CutAway::word`]; a mesh a candidate's mark carries to (its
/// mesh parent, or its mesh children) carries the record too, with only
/// its own fade-all-the-way bit.
#[derive(Component, Debug, Clone)]
pub struct CutAway {
    /// The node's cut-away word ([`CutAwayMesh::word`]).
    pub word: u32,
    /// The node mesh's triangles in the node's space (m), for the kinds
    /// whose callbacks walk them.
    pub triangles: Option<Arc<Vec<[Vec3; 3]>>>,
    /// The node's bounding sphere centre in its own space and the box
    /// radius the detector meets (the length of the box's half extents).
    pub centre: Vec3,
    pub radius: f32,
    /// The mesh parent a mark carries to (`None` without the carry bit or
    /// without a mesh parent).
    pub parent: Option<Entity>,
    /// The mesh children a mark carries to (empty without the carry bit).
    pub children: Vec<Entity>,
    /// The node's drawn part entities.
    pub parts: Vec<Entity>,
    /// The detector's mark.
    pub cut: bool,
    /// The fade amount, 0 solid to [`CutAway::target`].
    pub fade: f32,
    /// What the amount moves to ([`fade_target`]).
    pub target: f32,
    /// The visibility mask the last fade step left
    /// ([`crate::scene_nodes::MAIN`], [`crate::scene_nodes::DITHERED`],
    /// [`crate::scene_nodes::PARTIAL`]).
    pub mask: u32,
}

impl CutAway {
    /// A record for a node with cut-away word `word`.
    pub fn new(
        word: u32,
        triangles: Option<Arc<Vec<[Vec3; 3]>>>,
        centre: Vec3,
        half: Vec3,
    ) -> Self {
        Self {
            word,
            triangles,
            centre,
            radius: (half.x * half.x + half.z * half.z + half.y * half.y).sqrt(),
            parent: None,
            children: Vec::new(),
            parts: Vec::new(),
            cut: false,
            fade: 0.0,
            target: fade_target(word),
            mask: crate::scene_nodes::MAIN,
        }
    }

    /// The node's cut-away kind (`word & 0x7000_0000`).
    pub fn kind(&self) -> u32 {
        self.word & Node::CUT_AWAY_KIND
    }

    /// Whether the detector tests the node.
    pub fn candidate(&self) -> bool {
        self.word & Node::CUT_AWAY != 0
    }
}

/// The cut-away records of a spawned model: one per node that is a
/// candidate or that a candidate's mark carries to, with its relations
/// resolved to `entities` (the model's node entities) and its drawn parts
/// `parts_of`. Returns `(node index, record)` pairs.
pub fn records(
    model: &Model,
    entities: &[Entity],
    parts_of: &[Vec<Entity>],
) -> Vec<(usize, CutAway)> {
    let is_mesh = |i: usize| model.nodes[i].mesh || !model.nodes[i].parts.is_empty();
    let word = |i: usize| model.nodes[i].cut_away.as_ref().map_or(0, |c| c.word);
    let mut wanted = vec![false; model.nodes.len()];
    for (i, node) in model.nodes.iter().enumerate() {
        let w = word(i);
        if w & Node::CUT_AWAY == 0 {
            continue;
        }
        wanted[i] = true;
        if w & Node::CUT_AWAY_PARENT != 0
            && let Some(p) = node.parent.filter(|&p| p < model.nodes.len() && is_mesh(p))
        {
            wanted[p] = true;
        }
        if w & Node::CUT_AWAY_CHILDREN != 0 {
            for (c, child) in model.nodes.iter().enumerate() {
                if child.parent == Some(i) && is_mesh(c) {
                    wanted[c] = true;
                }
            }
        }
    }
    let mut out = Vec::new();
    for (i, node) in model.nodes.iter().enumerate() {
        if !wanted[i] || node.placeholder {
            continue;
        }
        let w = word(i);
        let triangles = node.cut_away.as_ref().and_then(|c| c.triangles.clone());
        let mut record = CutAway::new(w, triangles, node.bounds.0, node.half_extents);
        if w & Node::CUT_AWAY != 0 {
            if w & Node::CUT_AWAY_PARENT != 0 {
                record.parent = node
                    .parent
                    .filter(|&p| p < model.nodes.len() && is_mesh(p))
                    .map(|p| entities[p]);
            }
            if w & Node::CUT_AWAY_CHILDREN != 0 {
                record.children = (0..model.nodes.len())
                    .filter(|&c| model.nodes[c].parent == Some(i) && is_mesh(c))
                    .map(|c| entities[c])
                    .collect();
            }
        }
        record.parts = parts_of.get(i).cloned().unwrap_or_default();
        out.push((i, record));
    }
    out
}

/// `a · b` summed as the engine sums it: `(x·x′ + y·y′) + z·z′`.
fn dot(a: Vec3, b: Vec3) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

/// The overhead probe's mesh callback: the nearest triangle the ray from
/// `origin` along the unit `direction` meets within `length`, and the
/// distance to it. Only a triangle facing the ray counts (its edge
/// determinant at least [`CALLBACK_EPSILON`]); the barycentric tests are
/// the unnormalised ones, and each hit shortens the reach for the
/// triangles after it, as the original's walk does. All in the node's
/// space.
pub fn ray_cast(
    triangles: &[[Vec3; 3]],
    origin: Vec3,
    direction: Vec3,
    length: f32,
) -> Option<(usize, f32)> {
    let mut reach = length;
    let mut found = None;
    for (index, [v0, v1, v2]) in triangles.iter().copied().enumerate() {
        let e1 = v1 - v0;
        let e2 = v2 - v0;
        let p = direction.cross(e2);
        let det = dot(p, e1);
        // A NaN determinant goes on, as the original's unordered branch does.
        if det < CALLBACK_EPSILON {
            continue;
        }
        let s = origin - v0;
        let u = dot(s, p);
        if u < 0.0 || det < u {
            continue;
        }
        let q = s.cross(e1);
        let v = dot(direction, q);
        if v < 0.0 || det < u + v {
            continue;
        }
        let t = dot(e2, q) / det;
        if !(t > 0.0) || !(t < reach) {
            continue;
        }
        reach = t;
        found = Some((index, t));
    }
    found
}

/// The point of triangle `v` nearest `p` (the triangle's plane holds the
/// answer; `p` off the plane is first dropped onto it).
fn closest_on_triangle(p: Vec3, [a, b, c]: [Vec3; 3]) -> Vec3 {
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let denom = 1.0 / (va + vb + vc);
    a + ab * (vb * denom) + ac * (vc * denom)
}

/// The camera segment's mesh callback: a sphere of `radius` swept from
/// `origin` along the unit `direction` for `length`, against each triangle
/// whose plane (the face normal of its winding) the sphere's centre starts
/// on or in front of and moves toward, and reaches within the radius by
/// the segment's end. The sphere's centre where it first touches the plane
/// (the segment's end when it moves along the plane) is taken to the
/// triangle's nearest point, and the sweep's distance to that point is the
/// hit. Each hit shortens the sweep for the triangles after it. Returns
/// the nearest triangle and the distance, in the node's space.
///
/// The original keeps each triangle's plane with the mesh and finds the
/// nearest point with approximate reciprocals; Exarust takes the plane
/// from the triangle's winding and divides exactly.
pub fn sphere_cast(
    triangles: &[[Vec3; 3]],
    origin: Vec3,
    direction: Vec3,
    length: f32,
    radius: f32,
) -> Option<(usize, f32)> {
    let mut reach = length;
    let mut found = None;
    for (index, triangle) in triangles.iter().copied().enumerate() {
        let [v0, v1, v2] = triangle;
        let normal = (v1 - v0).cross(v2 - v0).normalize_or_zero();
        let plane = dot(normal, v0);
        let height = dot(origin, normal) - plane;
        if !(0.0 <= height) {
            continue;
        }
        let approach = dot(direction, normal) * reach;
        if !(approach <= 0.0) {
            continue;
        }
        let at_end = height + approach;
        if !(at_end <= radius) {
            continue;
        }
        let travel = if CALLBACK_EPSILON <= approach.abs() {
            (-(height - radius) / approach) * reach
        } else {
            reach
        };
        let contact = origin + direction * travel;
        let nearest = closest_on_triangle(contact, triangle);
        let off = nearest - contact;
        let along = dot(off, direction);
        let side = off - direction * along;
        let apart = dot(side, side);
        if !(apart < radius * radius) {
            continue;
        }
        let t =
            (dot(nearest, direction) - dot(origin, direction)) - (radius * radius - apart).sqrt();
        if !(0.0 < t) || !(t < reach) {
            continue;
        }
        reach = t;
        found = Some((index, t));
    }
    found
}

/// Inserts the cut-away records of a spawned model on its node entities,
/// and the camera wall probe's obstacles.
pub fn attach(
    commands: &mut Commands,
    model: &Model,
    entities: &[Entity],
    parts_of: &[Vec<Entity>],
) {
    for (i, record) in records(model, entities, parts_of) {
        commands.entity(entities[i]).insert(record);
    }
    for (i, node) in model.nodes.iter().enumerate() {
        let parts = parts_of.get(i).cloned().unwrap_or_default();
        if let Some(obstacle) = wall_obstacle(node, parts) {
            commands.entity(entities[i]).insert(obstacle);
        }
    }
}

/// A mesh the camera's wall probe meets ([`CutAwayMesh::probe`]): its
/// kind (`word & 0x7000_0000`), its triangles and its bounding sphere in
/// the node's space (metres), and its drawn parts. The probe asks a mesh
/// of kind `0x5…` its swept-sphere callback ([`sphere_cast`]) and any
/// other its ray callback ([`ray_cast`]). Rules: `docs/PLAYER.md`
/// (Camera).
#[derive(Component, Debug, Clone)]
pub struct WallObstacle {
    pub kind: u32,
    pub triangles: Arc<Vec<[Vec3; 3]>>,
    pub centre: Vec3,
    pub radius: f32,
    pub parts: Vec<Entity>,
}

/// The kind the wall probe asks with the swept sphere.
pub const WALL_SPHERE_KIND: u32 = 0x5000_0000;

impl WallObstacle {
    /// Whether the probe meets the mesh: the segment from `start` along
    /// the unit `direction` for `length` (world space) must meet the
    /// bounding sphere, then the callback in the node's space (`frame` its
    /// world frame): a kind `0x5…` mesh is swept by a ball of `radius` from
    /// the segment's far end back toward the start, any other kind is
    /// asked the one-sided ray from the start (the ball's radius unused).
    pub fn meets(
        &self,
        frame: &GlobalTransform,
        start: Vec3,
        direction: Vec3,
        length: f32,
        radius: f32,
    ) -> bool {
        let segment = direction * length;
        let centre = frame.transform_point(self.centre);
        if !exa_physics::intersect::segment_sphere(start, segment, centre, self.radius) {
            return false;
        }
        let inverse = frame.affine().inverse();
        let turn = frame.rotation().inverse();
        if self.kind == WALL_SPHERE_KIND {
            let from = inverse.transform_point3(start + segment);
            sphere_cast(&self.triangles, from, turn * -direction, length, radius).is_some()
        } else {
            let from = inverse.transform_point3(start);
            ray_cast(&self.triangles, from, turn * direction, length).is_some()
        }
    }
}

/// The wall probe's obstacle of a model node: a mesh whose record asks for
/// the probe and keeps its triangles, that is not a placeholder and that
/// no simulated body owns (the original skips a mesh with an owner).
pub fn wall_obstacle(node: &exa_assets::ModelNode, parts: Vec<Entity>) -> Option<WallObstacle> {
    let cut: &CutAwayMesh = node.cut_away.as_deref()?;
    if !cut.probe || node.placeholder || node.rigid.is_some() {
        return None;
    }
    let triangles = cut.triangles.clone()?;
    let half = node.half_extents;
    Some(WallObstacle {
        kind: cut.word & Node::CUT_AWAY_KIND,
        triangles,
        centre: node.bounds.0,
        radius: (half.x * half.x + half.z * half.z + half.y * half.y).sqrt(),
        parts,
    })
}

/// A grid set piece's cut-away record when its node is a candidate: the
/// grid spawns pieces one node at a time, so the carries to the parent and
/// the children are not resolved (no shipped set piece is known to use
/// them).
pub fn grid_record(node: &exa_assets::ModelNode, parts: Vec<Entity>) -> Option<CutAway> {
    let cut: &CutAwayMesh = node.cut_away.as_deref()?;
    if cut.word & Node::CUT_AWAY == 0 || node.placeholder {
        return None;
    }
    let mut record = CutAway::new(
        cut.word,
        cut.triangles.clone(),
        node.bounds.0,
        node.half_extents,
    );
    record.parts = parts;
    Some(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A square of side 2 at height `y`, facing up (+Y).
    fn floor(y: f32) -> Vec<[Vec3; 3]> {
        let a = Vec3::new(-1.0, y, -1.0);
        let b = Vec3::new(1.0, y, -1.0);
        let c = Vec3::new(1.0, y, 1.0);
        let d = Vec3::new(-1.0, y, 1.0);
        // Wound so that (b − a) × (c − a) points up.
        vec![[a, c, b], [a, d, c]]
    }

    /// A candidate without the fade-all-the-way bit settles at ½, drawn
    /// settled; with it, it goes all the way and is not drawn.
    #[test]
    fn the_default_target_settles_at_half() {
        use crate::scene_nodes::{MAIN, PARTIAL, SEEN, fade_step};
        let mut fade = 0.0;
        let mask = fade_step(MAIN | SEEN, true, &mut fade, fade_target(0x2000_0000), 300);
        assert_eq!((fade, mask), (0.5, SEEN | PARTIAL));
        let mut fade = 0.0;
        let mask = fade_step(MAIN | SEEN, true, &mut fade, fade_target(0x2800_0000), 400);
        assert_eq!((fade, mask), (1.0, SEEN));
    }

    #[test]
    fn the_targets_are_half_and_whole() {
        assert_eq!(fade_target(0x2000_0000), 0.5);
        assert_eq!(fade_target(0x2800_0000), 1.0);
        assert_eq!(fade_target(0), 0.5);
    }

    #[test]
    fn the_overhead_ray_meets_a_roof_from_above_only() {
        let roof = floor(3.0);
        let down = Vec3::NEG_Y;
        let hit = ray_cast(&roof, Vec3::new(0.2, 10.0, 0.3), down, 10.0).unwrap();
        assert!((hit.1 - 7.0).abs() < 1e-5, "{hit:?}");
        // Out of reach.
        assert!(ray_cast(&roof, Vec3::new(0.2, 10.0, 0.3), down, 6.0).is_none());
        // Beside the roof.
        assert!(ray_cast(&roof, Vec3::new(1.5, 10.0, 0.3), down, 10.0).is_none());
        // From below the roof faces away: no hit.
        assert!(ray_cast(&roof, Vec3::new(0.2, 0.0, 0.3), Vec3::Y, 10.0).is_none());
    }

    #[test]
    fn the_nearest_of_two_layers_is_kept() {
        let mut two = floor(2.0);
        two.extend(floor(5.0));
        let hit = ray_cast(&two, Vec3::new(0.0, 10.0, 0.1), Vec3::NEG_Y, 10.0).unwrap();
        assert!(hit.0 >= 2 && (hit.1 - 5.0).abs() < 1e-5, "{hit:?}");
    }

    #[test]
    fn the_swept_sphere_meets_a_face_it_moves_onto() {
        let roof = floor(3.0);
        let down = Vec3::NEG_Y;
        // Straight down onto the middle: the sphere touches at 3.5 m.
        let hit = sphere_cast(&roof, Vec3::new(0.0, 10.0, 0.0), down, 10.0, CAST_RADIUS).unwrap();
        assert!((hit.1 - 6.5).abs() < 1e-4, "{hit:?}");
        // Past the edge by less than the radius: the rim still touches.
        let rim = sphere_cast(&roof, Vec3::new(1.3, 10.0, 0.0), down, 10.0, CAST_RADIUS).unwrap();
        assert!(rim.1 > 6.5 && rim.1 < 7.0, "{rim:?}");
        // Past it by more than the radius: no hit.
        assert!(sphere_cast(&roof, Vec3::new(1.6, 10.0, 0.0), down, 10.0, CAST_RADIUS).is_none());
        // Stopping short of the plane by more than the radius: no hit.
        assert!(sphere_cast(&roof, Vec3::new(0.0, 10.0, 0.0), down, 6.0, CAST_RADIUS).is_none());
        // Moving away from the face: no hit.
        assert!(sphere_cast(&roof, Vec3::new(0.0, 0.0, 0.0), Vec3::Y, 10.0, CAST_RADIUS).is_none());
    }

    #[test]
    fn a_slanted_segment_through_an_awning_hits() {
        let awning = floor(3.0);
        let from = Vec3::new(-8.0, 12.0, 0.0);
        let to = Vec3::new(0.5, 1.5, 0.0);
        let dir = (to - from).normalize();
        assert!(sphere_cast(&awning, from, dir, (to - from).length(), CAST_RADIUS).is_some());
    }

    /// The shipped cut-away candidates: Objlib and Resource nodes whose
    /// second flag word holds the candidate bit, by pack and kind, and how
    /// many fade all the way. Prints the Resource ones (level meshes).
    #[test]
    fn shipped_cut_away_candidates() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let mut found = std::collections::BTreeMap::<(String, u32), usize>::new();
        let mut full = 0;
        let mut carries = std::collections::BTreeMap::<u32, usize>::new();
        let mut levels = Vec::new();
        for kind in [exa_assets::PackKind::Objlib, exa_assets::PackKind::Resource] {
            let Some(pack) = game.pack(kind) else {
                continue;
            };
            for entry in pack.entries() {
                if kind == exa_assets::PackKind::Resource && !entry.name.ends_with(".rfc") {
                    continue;
                }
                let Ok(scene) = exa_formats::rfc::Scene::parse(pack.bytes(entry)) else {
                    continue;
                };
                for node in &scene.nodes {
                    if node.flags2 & Node::CUT_AWAY == 0 {
                        continue;
                    }
                    *found
                        .entry((format!("{kind:?}"), node.flags2 & Node::CUT_AWAY_KIND))
                        .or_default() += 1;
                    if node.flags2 & Node::CUT_AWAY_FULL != 0 {
                        full += 1;
                    }
                    *carries
                        .entry(node.flags2 & (Node::CUT_AWAY_PARENT | Node::CUT_AWAY_CHILDREN))
                        .or_default() += 1;
                    if kind == exa_assets::PackKind::Resource {
                        // Where it stands in the level (m): its frame chain.
                        let mut frame = Mat4::IDENTITY;
                        let mut at = Some(node);
                        for _ in 0..scene.nodes.len() {
                            let Some(n) = at else { break };
                            frame =
                                exa_assets::transform_from_rows(&n.transform).to_matrix() * frame;
                            at = n
                                .parent
                                .and_then(|p| p.checked_sub(1))
                                .and_then(|p| scene.nodes.get(p));
                        }
                        let centre = frame.transform_point3(Vec3::from(node.bounds_centre));
                        let index = scene.nodes.iter().position(|n| std::ptr::eq(n, node));
                        let children: Vec<&str> = scene
                            .nodes
                            .iter()
                            .filter(|c| index.is_some() && c.parent == index.map(|i| i + 1))
                            .map(|c| c.name.as_str())
                            .collect();
                        levels.push(format!(
                            "{}:{} {:#x} at {centre:.1} half {:?} children {children:?}",
                            entry.name, node.name, node.flags2, node.bounds_half_extents
                        ));
                    }
                }
            }
        }
        eprintln!("candidates {found:?}; fade all the way {full}; carries {carries:?}");
        eprintln!("resource candidates: {levels:#?}");
        let objlib: usize = found
            .iter()
            .filter(|((p, _), _)| p == "Objlib")
            .map(|(_, n)| n)
            .sum();
        assert_eq!(objlib, 142);
        assert_eq!(levels.len(), 38);
    }

    /// A ray-kind obstacle meets the probe on the faces turned to its start;
    /// a sphere-kind one is swept from the far end back and meets the faces
    /// turned to that end; a segment that misses the bounding sphere asks
    /// nothing.
    #[test]
    fn the_wall_probe_asks_the_ray_or_the_swept_sphere_by_kind() {
        // A square 5 m out along +z, wound to face +z (toward the far end)
        // or -z (toward the start).
        let square = |toward_start: bool| {
            let (a, b, c, d) = (
                Vec3::new(-1.0, -1.0, 5.0),
                Vec3::new(1.0, -1.0, 5.0),
                Vec3::new(1.0, 1.0, 5.0),
                Vec3::new(-1.0, 1.0, 5.0),
            );
            let tris = if toward_start {
                vec![[a, c, b], [a, d, c]]
            } else {
                vec![[a, b, c], [a, c, d]]
            };
            Arc::new(tris)
        };
        let obstacle = |kind, toward_start| WallObstacle {
            kind,
            triangles: square(toward_start),
            centre: Vec3::new(0.0, 0.0, 5.0),
            radius: 1.5,
            parts: Vec::new(),
        };
        let frame = GlobalTransform::IDENTITY;
        let ask = |o: &WallObstacle, start: Vec3| o.meets(&frame, start, Vec3::Z, 38.5, 0.3);
        assert!(ask(&obstacle(0x1000_0000, true), Vec3::ZERO));
        assert!(!ask(&obstacle(0x1000_0000, false), Vec3::ZERO));
        assert!(ask(&obstacle(WALL_SPHERE_KIND, false), Vec3::ZERO));
        assert!(!ask(&obstacle(WALL_SPHERE_KIND, true), Vec3::ZERO));
        // Wide of the bounding sphere: nothing.
        assert!(!ask(&obstacle(0x1000_0000, true), Vec3::new(5.0, 0.0, 0.0)));
        // A moved node: the callbacks run in its space.
        let moved = GlobalTransform::from(Transform::from_xyz(3.0, 0.0, 0.0));
        let o = obstacle(0x1000_0000, true);
        assert!(o.meets(&moved, Vec3::new(3.0, 0.0, 0.0), Vec3::Z, 38.5, 0.3));
        assert!(!o.meets(&moved, Vec3::ZERO, Vec3::Z, 38.5, 0.3));
    }

    /// The camera wall probe's obstacles in the shipped packs: mesh nodes
    /// whose second flag word holds the probe's kind bit and the scene
    /// walks' mesh bit, by pack and kind (`0x1…` asked by the ray
    /// callback, `0x5…` by the swept sphere), and the levels that hold
    /// them.
    #[test]
    fn shipped_wall_probe_obstacles() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let mut found = std::collections::BTreeMap::<(String, u32), usize>::new();
        let mut levels = std::collections::BTreeMap::<String, usize>::new();
        for kind in [exa_assets::PackKind::Objlib, exa_assets::PackKind::Resource] {
            let Some(pack) = game.pack(kind) else {
                continue;
            };
            for entry in pack.entries() {
                let Ok(scene) = exa_formats::rfc::Scene::parse(pack.bytes(entry)) else {
                    continue;
                };
                for node in &scene.nodes {
                    if node.kind != exa_formats::rfc::NODE_MESH || !CutAwayMesh::probes(node.flags2)
                    {
                        continue;
                    }
                    *found
                        .entry((format!("{kind:?}"), node.flags2 & Node::CUT_AWAY_KIND))
                        .or_default() += 1;
                    if kind == exa_assets::PackKind::Resource {
                        *levels.entry(entry.name.clone()).or_default() += 1;
                        {
                            let mut frame = Mat4::IDENTITY;
                            let mut at = Some(node);
                            for _ in 0..scene.nodes.len() {
                                let Some(n) = at else { break };
                                frame = exa_assets::transform_from_rows(&n.transform).to_matrix()
                                    * frame;
                                at = n
                                    .parent
                                    .and_then(|p| p.checked_sub(1))
                                    .and_then(|p| scene.nodes.get(p));
                            }
                            let centre = frame.transform_point3(Vec3::from(node.bounds_centre));
                            eprintln!(
                                "obstacle {}:{} {:#x} at {centre:.1} half {:?}",
                                entry.name, node.name, node.flags2, node.bounds_half_extents
                            );
                        }
                    }
                }
            }
        }
        eprintln!("wall probe obstacles {found:?}; levels {levels:?}");
        let count = |pack: &str| -> usize {
            found
                .iter()
                .filter(|((p, _), _)| p == pack)
                .map(|(_, n)| n)
                .sum()
        };
        assert_eq!(count("Resource"), 171);
        assert!(levels.contains_key("arena7.rfc") && levels.contains_key("exanima08.rfc"));
        assert!(count("Objlib") > 0);
    }
}
