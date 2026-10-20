//! Skin weights for body-part meshes, worked out from their seams as the
//! original does when it loads a character.
//!
//! Body parts are authored as separate meshes in their bone's space. Each
//! carries invisible seam triangles (`_null`, flagged with a seam class in
//! the triangle flags) around the joints it shares with its neighbours. The
//! loop where seam triangles meet the visible skin is shared, vertex for
//! vertex, with the neighbouring part. Weighting that loop half to each bone,
//! and blending towards the neighbour with distance from the loop, makes the
//! skinned body watertight. Evidence: `docs/formats/RSQ.md` (Skinning).

use std::collections::{HashMap, HashSet};

use bevy::math::Vec3;
use exa_formats::rfc::Mesh;

/// Seam classes in the triangle flags' high byte, widest first, with the
/// multiplier on squared distance (cm²) that reaches 1 at the falloff
/// radius (20, 15, 10 and 5 cm).
const SEAM_CLASSES: [(u32, f32); 4] = [
    (0xC000_0000, 0.0025),
    (0x3000_0000, 0.004_444),
    (0x0C00_0000, 0.01),
    (0x0300_0000, 0.04),
];
const WIDE_SEAM: u32 = 0xC000_0000;
/// Neighbouring seam groups a vertex blends towards, at most.
const MAX_GROUPS: usize = 3;
const CM_PER_METRE: f32 = 100.0;

/// A joint the part shares with a neighbouring bone, at `pivot` in the
/// part's space (metres): the part's origin for its parent, a child's
/// translation for each child.
#[derive(Debug, Clone, Copy)]
pub struct Joint {
    pub bone: u16,
    pub pivot: Vec3,
}

/// Up to four (bone, weight) pairs per vertex, weights summing to 1.
pub type VertexSkin = [(u16, f32); 4];

struct Group {
    /// Boundary edges between seam and skin: endpoints (cm) and falloff.
    edges: Vec<(Vec3, Vec3, f32)>,
    centre: Vec3,
    /// Position ids covered by the seam (and the ring grown around it).
    vertices: HashSet<u32>,
    neighbour: Option<u16>,
}

fn falloff(flags: u32) -> f32 {
    SEAM_CLASSES
        .iter()
        .find(|(mask, _)| flags & mask != 0)
        .map_or(0.0025, |&(_, f)| f)
}

/// Squared distance from `p` to segment `a`–`b`.
fn segment_distance_sq(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    let t = if len_sq > 0.0 {
        ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + ab * t - p).length_squared()
}

/// Blend factors from the normalised distances `m` (< 1) to the nearest
/// seam groups: `1 − √m`, damped so several groups never sum past one.
fn group_factors(m: &[f32]) -> Vec<f32> {
    let w: Vec<f32> = m
        .iter()
        .map(|&m| if m <= 1e-6 { 1.0 } else { 1.0 - m.sqrt() })
        .collect();
    let s: f32 = w.iter().sum();
    let q: f32 = w.iter().map(|w| w * w).sum();
    if q <= 0.0 {
        return w;
    }
    let k = if s <= 1.0 { s / q } else { 1.0 / q };
    w.iter().map(|&w| w.min(w * w * k)).collect()
}

/// Skin weights of every vertex of `mesh`, a part of bone `own` meeting its
/// neighbours at `joints`.
pub fn auto_skin(mesh: &Mesh, own: u16, joints: &[Joint]) -> Vec<VertexSkin> {
    let rigid = [(own, 1.0), (own, 0.0), (own, 0.0), (own, 0.0)];
    let tri_count = mesh.indices.len() / 3;
    let Some(flags) = mesh
        .triangle_flags
        .as_ref()
        .filter(|f| f.len() >= tri_count)
    else {
        return vec![rigid; mesh.positions.len()];
    };
    let pid = |v: u32| mesh.position_ids[v as usize];
    let pos = |v: u32| Vec3::from(mesh.positions[v as usize]) * CM_PER_METRE;

    // Triangles around each position-id edge.
    let mut by_edge: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for t in 0..tri_count {
        let tri = &mesh.indices[t * 3..t * 3 + 3];
        for k in 0..3 {
            let (a, b) = (pid(tri[k]), pid(tri[(k + 1) % 3]));
            by_edge.entry((a.min(b), a.max(b))).or_default().push(t);
        }
    }
    let seam = |t: usize| flags.get(t).is_some_and(|f| f & WIDE_SEAM != 0);

    // Connected seam regions and their boundary loops.
    let mut groups = Vec::new();
    let mut seen = vec![false; tri_count];
    for start in 0..tri_count {
        if seen[start] || !seam(start) {
            continue;
        }
        let mut stack = vec![start];
        seen[start] = true;
        let mut tris = Vec::new();
        while let Some(t) = stack.pop() {
            tris.push(t);
            let tri = &mesh.indices[t * 3..t * 3 + 3];
            for k in 0..3 {
                let (a, b) = (pid(tri[k]), pid(tri[(k + 1) % 3]));
                for &n in &by_edge[&(a.min(b), a.max(b))] {
                    if !seen[n] && seam(n) {
                        seen[n] = true;
                        stack.push(n);
                    }
                }
            }
        }
        let mut edges = Vec::new();
        let mut vertices = HashSet::new();
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for &t in &tris {
            let tri = &mesh.indices[t * 3..t * 3 + 3];
            for k in 0..3 {
                let (va, vb) = (tri[k], tri[(k + 1) % 3]);
                let (a, b) = (pid(va), pid(vb));
                vertices.insert(a);
                if let Some(&other) = by_edge[&(a.min(b), a.max(b))].iter().find(|&&n| !seam(n)) {
                    let (pa, pb) = (pos(va), pos(vb));
                    edges.push((pa, pb, falloff(flags[t] | flags[other])));
                    lo = lo.min(pa).min(pb);
                    hi = hi.max(pa).max(pb);
                }
            }
        }
        if edges.is_empty() {
            continue;
        }
        // Grow over the narrower seam classes touching the region.
        for mask in [0xF000_0000u32, 0xFC00_0000] {
            let ring: Vec<u32> = (0..tri_count)
                .filter(|&t| flags[t] & mask != 0)
                .filter(|&t| {
                    mesh.indices[t * 3..t * 3 + 3]
                        .iter()
                        .any(|&v| vertices.contains(&pid(v)))
                })
                .flat_map(|t| mesh.indices[t * 3..t * 3 + 3].iter().map(|&v| pid(v)))
                .collect();
            vertices.extend(ring);
        }
        groups.push(Group {
            edges,
            centre: (lo + hi) * 0.5,
            vertices,
            neighbour: None,
        });
    }

    // Each joint takes the nearest free seam region, nearest pairs first.
    let mut pairs: Vec<(f32, usize, usize)> = joints
        .iter()
        .enumerate()
        .flat_map(|(j, joint)| {
            let pivot = joint.pivot * CM_PER_METRE;
            groups
                .iter()
                .enumerate()
                .map(move |(g, group)| (group.centre.distance_squared(pivot), j, g))
        })
        .collect();
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut joint_done = vec![false; joints.len()];
    for (_, j, g) in pairs {
        if !joint_done[j] && groups[g].neighbour.is_none() {
            groups[g].neighbour = Some(joints[j].bone);
            joint_done[j] = true;
        }
    }

    // Seam class of the triangles around each position id.
    let mut class: HashMap<u32, u32> = HashMap::new();
    for (tri, &f) in mesh.indices.as_chunks::<3>().0.iter().zip(flags.iter()) {
        for &v in tri {
            *class.entry(pid(v)).or_default() |= f & 0xFF00_0000;
        }
    }

    let mut by_position: HashMap<u32, VertexSkin> = HashMap::new();
    (0..mesh.positions.len())
        .map(|v| {
            let id = pid(v as u32);
            *by_position.entry(id).or_insert_with(|| {
                let p = pos(v as u32);
                // Vertices of a seam region follow its fixed split.
                if let Some(g) = groups
                    .iter()
                    .find(|g| g.neighbour.is_some() && g.vertices.contains(&id))
                {
                    let c = class.get(&id).copied().unwrap_or(0);
                    let share = if c & WIDE_SEAM != 0 {
                        0.5
                    } else if c & 0x3000_0000 != 0 {
                        0.25
                    } else if c & 0x0C00_0000 != 0 {
                        0.125
                    } else {
                        0.0
                    };
                    let n = g.neighbour.expect("filtered");
                    return [(own, 1.0 - share), (n, share), (own, 0.0), (own, 0.0)];
                }
                // Others blend towards the nearest seams with distance.
                let mut near: Vec<(f32, u16)> = groups
                    .iter()
                    .filter_map(|g| {
                        let n = g.neighbour?;
                        let m = g
                            .edges
                            .iter()
                            .map(|&(a, b, f)| segment_distance_sq(p, a, b) * f)
                            .fold(f32::MAX, f32::min);
                        (m < 0.999).then_some((m, n))
                    })
                    .collect();
                near.sort_by(|a, b| a.0.total_cmp(&b.0));
                near.truncate(MAX_GROUPS);
                let factors = group_factors(&near.iter().map(|n| n.0).collect::<Vec<_>>());
                let mut skin = rigid;
                let mut own_weight = 1.0;
                for (k, (&(_, bone), f)) in near.iter().zip(&factors).enumerate() {
                    skin[k + 1] = (bone, 0.5 * f);
                    own_weight -= 0.5 * f;
                }
                skin[0] = (own, own_weight);
                skin
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A strip of quads along +X from 0 to 30 cm; the last quad is seam.
    fn strip() -> Mesh {
        let xs = [0.0f32, 0.1, 0.2, 0.3];
        let mut positions = Vec::new();
        for &x in &xs {
            positions.push([x, 0.0, 0.0]);
            positions.push([x, 0.1, 0.0]);
        }
        let mut indices = Vec::new();
        let mut flags = Vec::new();
        for q in 0..3u32 {
            let (a, b, c, d) = (q * 2, q * 2 + 1, q * 2 + 2, q * 2 + 3);
            indices.extend([a, c, b, b, c, d]);
            let f = if q == 2 { 0x8000_0000 } else { 1 };
            flags.extend([f, f]);
        }
        Mesh {
            normals: vec![[0.0, 0.0, 1.0]; positions.len()],
            position_ids: (0..positions.len() as u32).collect(),
            positions,
            uvs: None,
            colors: None,
            skin: None,
            indices,
            groups: vec![],
            triangle_flags: Some(flags),
            render_flags: 0,
            gi_points: vec![],
            position_count: 8,
            smoothing: vec![],
            ..Mesh::default()
        }
    }

    #[test]
    fn seam_loop_splits_evenly_and_blend_fades() {
        let skin = auto_skin(
            &strip(),
            12,
            &[Joint {
                bone: 13,
                pivot: Vec3::new(0.3, 0.05, 0.0),
            }],
        );
        let weight = |v: usize, bone: u16| -> f32 {
            skin[v]
                .iter()
                .filter(|(b, _)| *b == bone)
                .map(|(_, w)| w)
                .sum()
        };
        // The loop at x = 20 cm sits on the seam: half and half.
        assert!((weight(4, 13) - 0.5).abs() < 1e-5);
        // 10 cm inside: blended, less than half.
        let mid = weight(2, 13);
        assert!(mid > 0.0 && mid < 0.5, "{mid}");
        // 20 cm inside reaches the falloff radius: own bone only.
        assert!(weight(0, 13) < 1e-4);
        for s in &skin {
            let total: f32 = s.iter().map(|(_, w)| w).sum();
            assert!((total - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn meshes_without_flags_stay_rigid() {
        let mut m = strip();
        m.triangle_flags = None;
        let skin = auto_skin(&m, 3, &[]);
        assert!(skin.iter().all(|s| s[0] == (3, 1.0)));
    }

    /// Compares with weights measured on the shipped body (run with
    /// `cargo test -p exa_assets -- --ignored` and the game installed).
    #[test]
    #[ignore]
    fn upper_arm_of_the_shipped_body() {
        use exa_formats::rfc::{NodeBody, Scene};
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = crate::GameData::open(dir).unwrap();
        let bytes = game
            .pack(crate::PackKind::Resource)
            .and_then(|p| p.get("humanbase.rfc"))
            .unwrap();
        let scene = Scene::parse(bytes).unwrap();
        let (i, arm) = scene
            .nodes
            .iter()
            .enumerate()
            .find(|(_, n)| n.name == "12RUArm")
            .unwrap();
        let NodeBody::Mesh(mesh) = &arm.body else {
            panic!("not a mesh")
        };
        let mut joints = vec![];
        if let Some(p) = arm.parent {
            let bone = exa_formats::rsq::bone_index(&scene.nodes[p - 1].name).unwrap() as u16;
            joints.push(Joint {
                bone,
                pivot: Vec3::ZERO,
            });
        }
        for n in scene.nodes.iter().filter(|n| n.parent == Some(i + 1)) {
            if let Some(bone) = exa_formats::rsq::bone_index(&n.name) {
                joints.push(Joint {
                    bone: bone as u16,
                    pivot: Vec3::from(n.transform[3]),
                });
            }
        }
        let skin = auto_skin(mesh, 12, &joints);
        for x in [-27.1f32, -20.3, -12.2, -7.0, -3.1] {
            let (v, _) = mesh
                .positions
                .iter()
                .enumerate()
                .map(|(v, p)| (v, (p[0] * 100.0 - x).abs()))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap();
            println!("x {x}: {:?}", skin[v]);
        }
    }

    #[test]
    fn factors_never_overshoot() {
        let f = group_factors(&[0.0, 0.0]);
        assert!(f.iter().sum::<f32>() <= 1.0 + 1e-6);
        let f = group_factors(&[0.25]);
        assert!((f[0] - 0.5).abs() < 1e-6);
    }
}
