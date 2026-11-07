//! Skinned linked-body objects: the original's `_SKIN` build for a model
//! whose root mesh asks for it (render word `0x400000`), such as the sacks,
//! mattresses, flails and hanging items whose parts are linked rigid bodies.
//!
//! When the importer finishes a load, a root mesh (class `0x89`) with that
//! bit that is its own master is handed to the skin builder. The builder
//! makes one skin controller whose bones are the mesh nodes of the tree, in
//! the order its walk meets them: the root first, then at each node its
//! mesh children that carry the bit without `0x800000` (in child order),
//! before the walk enters them one by one. A node's bone keeps its node's
//! frame, and its inverse bind is the node's frame at the build turned back
//! into the skin's space. Each mesh's weights come from its stored skin
//! stream, whose bone numbers are these bone numbers (1-based); a mesh
//! without one gets the automatic seam weights against its parent and its
//! children. The tree's meshes are then appended into one `_SKIN` mesh
//! drawn in their place (their own draw is skipped and they cast nothing),
//! and each frame the controller moves the vertices by the bones' frames,
//! so the parts' bodies, swinging on their links, bend the one surface.
//!
//! Exarust keeps each mesh as its own skinned draw over the same joints
//! (the merge only puts the meshes into one buffer): [`plan`] numbers the
//! bones and [`inverse_binds`] gives each mesh its binds.

use bevy::math::{Affine3A, Mat4};
use exa_formats::rfc::{NodeBody, Scene};

/// Render-word bit: the mesh is part of a skinned object.
pub const SKINNED: u32 = 0x40_0000;
/// Render-word bit the builder sets on the root it has built; a child
/// carrying it is not walked.
pub const BUILT: u32 = 0x80_0000;
/// `flags2` bit that keeps a root from being built again.
pub const ALREADY_SKINNED: u32 = 0x8;

/// The class of a mesh node (the low twelve bits of its flags).
const MESH_CLASS: u32 = 0x89;

fn render_word(scene: &Scene, i: usize) -> Option<u32> {
    match &scene.nodes.get(i)?.body {
        NodeBody::Mesh(mesh) => Some(mesh.render_flags),
        _ => None,
    }
}

/// Whether node `i` is one of the walk's mesh children: class `0x89` with
/// the skinned bit and without the built bit.
fn walked(scene: &Scene, i: usize) -> bool {
    scene.nodes[i].flags & 0xfff == MESH_CLASS
        && render_word(scene, i).is_some_and(|w| w & (SKINNED | BUILT) == SKINNED)
}

/// Whether node `i` is a root the importer's finalise pass hands to the
/// skin builder: a root of the node list (no parent, or one out of range),
/// whose flags hold the mesh class bits, whose render word has
/// [`SKINNED`], that owns its data (no instance word) and is not built yet.
pub fn is_skin_root(scene: &Scene, i: usize) -> bool {
    let node = &scene.nodes[i];
    let root = node.parent.is_none_or(|p| p == 0 || p > scene.nodes.len());
    root && node.flags & MESH_CLASS == MESH_CLASS
        && node.instance.is_none()
        && node.flags2 & ALREADY_SKINNED == 0
        && render_word(scene, i).is_some_and(|w| w & SKINNED != 0)
}

/// One skinned object of a model: its bones and the bone of each node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinPlan {
    /// The node of each bone: bone `b` (1-based) is `joints[b]`; entry 0,
    /// which no weight names, is the root.
    pub joints: Vec<usize>,
    /// Each member node and its own bone, in the walk's order.
    pub members: Vec<(usize, u16)>,
}

impl SkinPlan {
    /// The bone of node `i`, when it is a member.
    pub fn bone_of(&self, i: usize) -> Option<u16> {
        self.members.iter().find(|(n, _)| *n == i).map(|(_, b)| *b)
    }
}

/// Every skinned object of `scene`, in root order.
pub fn plan(scene: &Scene) -> Vec<SkinPlan> {
    (0..scene.nodes.len())
        .filter(|&i| is_skin_root(scene, i))
        .map(|root| plan_root(scene, root))
        .collect()
}

/// The builder's walk from `root`: a node's walked children take their
/// bones before the walk enters the first of them.
fn plan_root(scene: &Scene, root: usize) -> SkinPlan {
    let len = scene.nodes.len();
    let mut children = vec![Vec::new(); len];
    for (i, node) in scene.nodes.iter().enumerate() {
        if let Some(p) = node.parent
            && (1..=len).contains(&p)
            && p - 1 != i
        {
            children[p - 1].push(i);
        }
    }
    let mut plan = SkinPlan {
        joints: vec![root, root],
        members: vec![(root, 1)],
    };
    let bone = |plan: &mut SkinPlan, i: usize| {
        if plan.bone_of(i).is_none() {
            plan.joints.push(i);
            let b = (plan.joints.len() - 1) as u16;
            plan.members.push((i, b));
        }
    };
    let mut stack = vec![root];
    // Depth first; a malformed cycle ends where a node repeats.
    let mut entered = vec![false; len];
    while let Some(at) = stack.pop() {
        if std::mem::replace(&mut entered[at], true) {
            continue;
        }
        let kids: Vec<usize> = children[at]
            .iter()
            .copied()
            .filter(|&c| walked(scene, c))
            .collect();
        for &c in &kids {
            bone(&mut plan, c);
        }
        stack.extend(kids.into_iter().rev());
    }
    plan
}

/// Node `i`'s frame in the model's space as the file stores it (its local
/// transforms composed up to its root).
pub fn rest_frame(scene: &Scene, i: usize) -> Mat4 {
    let len = scene.nodes.len();
    let mut m = Mat4::IDENTITY;
    let mut at = Some(i);
    for _ in 0..=len {
        let Some(n) = at else { break };
        let local = crate::model::transform_from_rows(&scene.nodes[n].transform).to_matrix();
        m = local * m;
        at = scene.nodes[n]
            .parent
            .filter(|&p| (1..=len).contains(&p) && p - 1 != n)
            .map(|p| p - 1);
    }
    m
}

/// The inverse binds of member `own`'s mesh, joint by joint (joint `b` is
/// bone `b`): the bone's frame at the build turned back, times the mesh's
/// own frame, so that at rest every bone moves the vertex to where its own
/// node puts it. Joint 0 is the identity.
pub fn inverse_binds(scene: &Scene, plan: &SkinPlan, own: usize) -> Vec<Mat4> {
    let own_frame = Affine3A::from_mat4(rest_frame(scene, own));
    plan.joints
        .iter()
        .enumerate()
        .map(|(b, &n)| {
            if b == 0 {
                Mat4::IDENTITY
            } else {
                Mat4::from(Affine3A::from_mat4(rest_frame(scene, n)).inverse() * own_frame)
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "prop_skin_test.rs"]
mod tests;
