//! Skinned linked-body objects in the world: a spawned model's meshes that
//! the original merges into a `_SKIN` mesh ([`exa_assets::prop_skin`]) are
//! drawn skinned to the object's own nodes, so the parts' bodies, swinging
//! on their links, bend one surface instead of moving rigid pieces.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::camera::visibility::DynamicSkinnedMeshBounds;
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::prelude::*;

pub struct PropSkinPlugin;

impl Plugin for PropSkinPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, bind_prop_skins);
    }
}

/// A part waiting for its skin: its skinned mesh, the entity of each joint
/// (joint `b` is bone `b`) and the joints' inverse binds.
#[derive(Component, Clone, Debug)]
pub struct PropSkinPart {
    pub mesh: Handle<Mesh>,
    pub joints: Vec<Entity>,
    pub inverse_binds: Arc<[Mat4]>,
}

/// Gives each part of a skinned node (`parts`, in the node's part order)
/// its skin over the model's node entities (`nodes`, in node order).
pub fn attach(
    commands: &mut Commands,
    skin: &exa_assets::PropSkin,
    parts: &[Entity],
    nodes: &[Entity],
) {
    let joints: Vec<Entity> = skin
        .joints
        .iter()
        .map(|&n| nodes.get(n).copied().unwrap_or(Entity::PLACEHOLDER))
        .collect();
    if joints.contains(&Entity::PLACEHOLDER) {
        return;
    }
    for (&part, mesh) in parts.iter().zip(&skin.parts) {
        commands.entity(part).insert(PropSkinPart {
            mesh: mesh.clone(),
            joints: joints.clone(),
            inverse_binds: skin.inverse_binds.clone(),
        });
    }
}

/// One bind asset per distinct bind list, by the list's address.
type SharedBinds = HashMap<usize, (Arc<[Mat4]>, Handle<SkinnedMeshInverseBindposes>)>;

/// Turns each waiting part into a skinned mesh, sharing one bind asset per
/// distinct bind list.
fn bind_prop_skins(
    mut commands: Commands,
    parts: Query<(Entity, &PropSkinPart), Without<SkinnedMesh>>,
    mut bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    mut shared: Local<SharedBinds>,
) {
    for (entity, part) in &parts {
        let key = Arc::as_ptr(&part.inverse_binds) as *const Mat4 as usize;
        let handle = match shared.get(&key) {
            Some((binds, handle)) if Arc::ptr_eq(binds, &part.inverse_binds) => handle.clone(),
            _ => {
                let handle = bindposes.add(SkinnedMeshInverseBindposes::from(
                    part.inverse_binds.to_vec(),
                ));
                shared.insert(key, (part.inverse_binds.clone(), handle.clone()));
                handle
            }
        };
        commands.entity(entity).try_insert((
            Mesh3d(part.mesh.clone()),
            SkinnedMesh {
                inverse_bindposes: handle,
                joints: part.joints.clone(),
            },
            // The parts swing away from the mesh's rest box: the box
            // follows the joints.
            DynamicSkinnedMeshBounds,
        ));
    }
}
