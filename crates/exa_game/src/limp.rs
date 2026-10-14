//! Bodies going limp: a body model asked to [`GoLimp`] becomes a simulated
//! ragdoll from its current pose, and stops being animated and steered.

use std::collections::HashMap;

use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_anim::{Animator, BodyPart, Motions};
use exa_assets::SkinnedPart;
use exa_physics::CharacterBody;
use exa_physics::body::{GoLimp, PartSetup, Ragdoll};

pub struct LimpPlugin;

impl Plugin for LimpPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, start_ragdolls);
    }
}

/// Half extent of a part without a mesh, metres.
const FALLBACK_HALF: f32 = 0.05;

#[allow(clippy::too_many_arguments)]
fn start_ragdolls(
    mut commands: Commands,
    motions: Option<Res<Motions>>,
    mut requests: Query<(Entity, &mut GoLimp), Without<Ragdoll>>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    parts: Query<(&BodyPart, &GlobalTransform)>,
    locals: Query<&Transform>,
    skinned: Query<(&SkinnedPart, &Mesh3d)>,
    controlled: Query<(), With<CharacterBody>>,
    meshes: Res<Assets<Mesh>>,
) {
    let Some(motions) = motions else { return };
    for (root, mut limp) in &mut requests {
        // Wait until the body has been placed and posed.
        if limp.delay > 0 {
            limp.delay -= 1;
            continue;
        }
        let found: Vec<(Entity, usize, GlobalTransform)> = children
            .iter_descendants(root)
            .filter_map(|e| parts.get(e).ok().map(|(p, g)| (e, p.bone, *g)))
            .collect();
        if found.is_empty() {
            continue;
        }
        let index: HashMap<Entity, usize> =
            found.iter().enumerate().map(|(i, f)| (f.0, i)).collect();
        let setup: Vec<PartSetup> = found
            .iter()
            .map(|&(entity, bone, world)| {
                let parent = parents
                    .iter_ancestors(entity)
                    .take_while(|&a| a != root)
                    .find_map(|a| index.get(&a).copied());
                let (centre, half) = part_box(entity, bone, &children, &skinned, &meshes);
                // Both points of a link are given in the part's rest frame;
                // the parent's is carried into the parent through the rest
                // placement of the part under it.
                let links = match (parent, motions.1.get(&bone)) {
                    (Some(p), Some(body)) => {
                        let to_parent = rest_under(entity, found[p].0, &parents, &parts, &locals);
                        body.links
                            .iter()
                            .map(|l| {
                                (
                                    Vec3::from_array(l.on_part),
                                    to_parent.transform_point3(Vec3::from_array(l.on_parent)),
                                    l.slack,
                                )
                            })
                            .collect()
                    }
                    _ => Vec::new(),
                };
                PartSetup {
                    bone,
                    entity,
                    parent,
                    world: world.affine(),
                    centre,
                    half,
                    links,
                }
            })
            .collect();
        let mut ragdoll = Ragdoll::new(&setup);
        ragdoll.set_velocity(limp.velocity);
        commands
            .entity(root)
            .insert(ragdoll)
            .remove::<(GoLimp, Animator)>();
        if let Some(character) = parents
            .iter_ancestors(root)
            .find(|&a| controlled.contains(a))
        {
            commands.entity(character).remove::<CharacterBody>();
        }
    }
}

/// Rest placement of `part` in the space of its ancestor `parent`: body
/// parts contribute their rest transforms, other nodes their current ones.
fn rest_under(
    part: Entity,
    parent: Entity,
    parents: &Query<&ChildOf>,
    parts: &Query<(&BodyPart, &GlobalTransform)>,
    locals: &Query<&Transform>,
) -> Affine3A {
    let mut placement = Affine3A::IDENTITY;
    let mut node = part;
    while node != parent {
        let local = match parts.get(node) {
            Ok((p, _)) => p.rest,
            Err(_) => locals.get(node).copied().unwrap_or_default(),
        };
        placement = local.compute_affine() * placement;
        match parents.get(node) {
            Ok(up) => node = up.parent(),
            Err(_) => break,
        }
    }
    placement
}

/// Box of a part's own skinned body meshes, in its space.
fn part_box(
    part: Entity,
    bone: usize,
    children: &Query<&Children>,
    skinned: &Query<(&SkinnedPart, &Mesh3d)>,
    meshes: &Assets<Mesh>,
) -> (Vec3, Vec3) {
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for child in children.get(part).into_iter().flat_map(|c| c.iter()) {
        let Ok((s, mesh)) = skinned.get(child) else {
            continue;
        };
        if s.bone as usize != bone {
            continue;
        }
        let Some(positions) = meshes
            .get(&mesh.0)
            .and_then(|m| m.attribute(Mesh::ATTRIBUTE_POSITION))
            .and_then(|a| a.as_float3())
        else {
            continue;
        };
        for p in positions {
            lo = lo.min(Vec3::from(*p));
            hi = hi.max(Vec3::from(*p));
        }
    }
    if lo.x > hi.x {
        return (Vec3::ZERO, Vec3::splat(FALLBACK_HALF));
    }
    ((lo + hi) * 0.5, (hi - lo) * 0.5)
}
