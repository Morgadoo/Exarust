//! Turns a cloth's draw passes into materials the renderer submits.
//!
//! A character's hair cloth (its record carries the [`HAIR_PASSES`] bit)
//! is drawn by its own draw procedure as two culled passes: front faces
//! culled first, then back faces ([`draw_hair_passes`]). The level's
//! cloths keep the single culling-off draw.

use bevy::prelude::*;
use bevy::render::render_resource::Face;
use exa_physics::cloth::Cloth;
use exa_render::material::ExaMaterial;

use crate::cloth::ClothInstance;

/// The cloth record bit whose cloths install the two-pass draw (the hair's
/// record has it; the level's does not).
pub const HAIR_PASSES: u32 = 0x4;

/// How far after its first pass the back-culled pass sorts on the
/// transparent phase (m): two ulps of the blended walk's largest sort
/// values, so the pass follows the first one at once, as the original
/// draws both in one call.
const SECOND_PASS_STEP: f32 = 0.5;

/// A hair cloth's two passes: its own entity draws `front` (front faces
/// culled), the child `back_draw` draws `back` (back faces culled) on the
/// same mesh.
#[derive(Component, Debug, Clone)]
pub struct HairPasses {
    pub front: Handle<ExaMaterial>,
    pub back: Handle<ExaMaterial>,
    pub back_draw: Entity,
}

/// The back-culled pass of a hair cloth, drawn after its front-culled one.
fn back_pass(front: &ExaMaterial) -> ExaMaterial {
    let mut back = front.clone();
    back.cloth_cull = Some(Face::Back);
    back.second_pass = true;
    back.draw_order = front.draw_order + SECOND_PASS_STEP;
    back
}

/// Draws every hair cloth as the original's hair draw procedure does: the
/// cloth's mesh with front faces culled, then again with back faces culled.
/// The cloth's entity takes the front-culled copy of its material and a
/// child draws the back-culled one; a material the entity is given later
/// (or a change to the front copy) is copied again.
#[allow(clippy::type_complexity)]
pub fn draw_hair_passes(
    mut commands: Commands,
    mut cloths: Query<(
        Entity,
        &ClothInstance,
        &Mesh3d,
        &mut MeshMaterial3d<ExaMaterial>,
        Option<&HairPasses>,
    )>,
    mut materials: ResMut<Assets<ExaMaterial>>,
    mut events: MessageReader<AssetEvent<ExaMaterial>>,
) {
    let modified: Vec<AssetId<ExaMaterial>> = events
        .read()
        .filter_map(|e| match e {
            AssetEvent::Modified { id } => Some(*id),
            _ => None,
        })
        .collect();
    for (entity, instance, mesh, mut material, passes) in &mut cloths {
        if instance.cloth.record.flags & HAIR_PASSES == 0 {
            continue;
        }
        match passes {
            Some(passes) if material.0 == passes.front => {
                // The front copy changed (blood, dirt, the walk's order):
                // the back pass follows it.
                if modified.contains(&passes.front.id())
                    && let Some(front) = materials.get(&passes.front)
                {
                    let back = back_pass(front);
                    let _ = materials.insert(passes.back.id(), back);
                }
            }
            _ => {
                let Some(source) = materials.get(&material.0) else {
                    continue;
                };
                let mut front = source.clone();
                front.cloth_cull = Some(Face::Front);
                let back = back_pass(&front);
                let front = materials.add(front);
                let back = materials.add(back);
                material.0 = front.clone();
                let back_draw = match passes {
                    Some(old) => {
                        commands
                            .entity(old.back_draw)
                            .insert(MeshMaterial3d(back.clone()));
                        old.back_draw
                    }
                    None => commands
                        .spawn((
                            Name::new("hair back pass"),
                            Mesh3d(mesh.0.clone()),
                            MeshMaterial3d(back.clone()),
                            Transform::IDENTITY,
                            ChildOf(entity),
                        ))
                        .id(),
                };
                commands.entity(entity).insert(HairPasses {
                    front,
                    back,
                    back_draw,
                });
            }
        }
    }
}

/// One material per pass the cloth submits. A double-sided cloth culls the
/// front and then the back. A single-sided cloth culls only the back.
pub fn cloth_draw_materials(base: &ExaMaterial, cloth: &Cloth) -> Vec<ExaMaterial> {
    base.cloth_draws(&cloth.submitted_culls())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec3;
    use bevy::render::render_resource::Face;
    use exa_render::material::ExaMaterialKey;

    #[test]
    fn a_mesh_cloth_submits_a_front_pass_and_a_back_pass() {
        let vertices = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let cloth = Cloth::from_mesh(&vertices, &[[0, 1, 3], [1, 2, 3]], &[0]);
        let draws = cloth_draw_materials(&ExaMaterial::default(), &cloth);
        assert_eq!(draws.len(), 2);
        assert_eq!(
            ExaMaterialKey::from(&draws[0]).cloth_cull(),
            Some(Face::Front)
        );
        assert_eq!(
            ExaMaterialKey::from(&draws[1]).cloth_cull(),
            Some(Face::Back)
        );

        let mut single = cloth;
        single.double_sided = false;
        let draws = cloth_draw_materials(&ExaMaterial::default(), &single);
        assert_eq!(draws.len(), 1);
        assert_eq!(
            ExaMaterialKey::from(&draws[0]).cloth_cull(),
            Some(Face::Back)
        );
    }

    /// A hair cloth (record bit 4) draws two culled passes, front faces
    /// culled on its own entity, then back faces on a child; a level cloth
    /// keeps its one material. A material given later is copied again.
    #[test]
    fn a_hair_cloth_draws_two_culled_passes() {
        use crate::cloth::ClothInstance;
        use exa_physics::cloth_record::ClothRecord;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<ExaMaterial>()
            .init_asset::<Mesh>()
            .add_systems(Update, draw_hair_passes);
        let vertices = [Vec3::ZERO, Vec3::X, Vec3::Y];
        let cloth = |record| {
            let mut c = Cloth::from_mesh(&vertices, &[[0, 1, 2]], &[0]);
            c.record = record;
            c
        };
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Mesh::new(
                bevy::mesh::PrimitiveTopology::TriangleList,
                Default::default(),
            ));
        let base = app
            .world_mut()
            .resource_mut::<Assets<ExaMaterial>>()
            .add(ExaMaterial {
                draw_order: -10.0,
                ..ExaMaterial::default()
            });
        let spawn = |app: &mut App, record| {
            app.world_mut()
                .spawn((
                    ClothInstance::new(cloth(record), mesh.clone(), vec![0, 1, 2]),
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(base.clone()),
                ))
                .id()
        };
        let hair = spawn(&mut app, ClothRecord::HAIR);
        let banner = spawn(&mut app, ClothRecord::LEVEL);
        app.update();
        let world = app.world();
        let passes = world.get::<HairPasses>(hair).expect("hair passes");
        let materials = world.resource::<Assets<ExaMaterial>>();
        let own = &world.get::<MeshMaterial3d<ExaMaterial>>(hair).unwrap().0;
        assert_eq!(*own, passes.front);
        let front = materials.get(own).unwrap();
        assert_eq!(ExaMaterialKey::from(front).cloth_cull(), Some(Face::Front));
        let back_handle = &world
            .get::<MeshMaterial3d<ExaMaterial>>(passes.back_draw)
            .unwrap()
            .0;
        let back = materials.get(back_handle).unwrap();
        assert_eq!(ExaMaterialKey::from(back).cloth_cull(), Some(Face::Back));
        assert!(back.second_pass && back.draw_order > front.draw_order);
        assert_eq!(world.get::<ChildOf>(passes.back_draw).unwrap().parent(), hair);
        assert!(world.get::<HairPasses>(banner).is_none());
        assert_eq!(world.get::<MeshMaterial3d<ExaMaterial>>(banner).unwrap().0, base);
        // A new material on the hair is split again, on the same child.
        let child = passes.back_draw;
        let other = app
            .world_mut()
            .resource_mut::<Assets<ExaMaterial>>()
            .add(ExaMaterial::default());
        app.world_mut()
            .get_mut::<MeshMaterial3d<ExaMaterial>>(hair)
            .unwrap()
            .0 = other.clone();
        app.update();
        let passes = app.world().get::<HairPasses>(hair).unwrap();
        assert_eq!(passes.back_draw, child);
        assert_ne!(
            app.world().get::<MeshMaterial3d<ExaMaterial>>(hair).unwrap().0,
            other
        );
    }
}
