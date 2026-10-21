//! Factory weapons: assembled from component models as a factory recipe and
//! an item's seeds choose, shaped by the recipe's deforms and painted per
//! material area like apparel.

use std::sync::Arc;

use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use exa_formats::factory::{Built, ComponentBox, Factory, Generated, WeaponSpec, deform_positions};
use exa_formats::objdb::{Appearance, Layer, Weapon};
use exa_formats::weapon_stats::{self, StatBytes};

use crate::PackKind;
use crate::model::{Model, ModelLoader, ModelNode};

const CM_PER_METRE: f32 = 100.0;

impl ModelLoader<'_> {
    /// A weapon built from factory `name` (`Factories.rpk`, without the `@`),
    /// with the item's weapon data when known; without it, a fresh recipe
    /// with zero seeds.
    pub fn factory_weapon(&mut self, name: &str, weapon: Option<&Weapon>) -> Option<Arc<Model>> {
        let key = factory_key(name, weapon);
        if let Some(m) = self.cached_model(&key) {
            return m;
        }
        let built = self.build_factory_weapon(name, weapon, &key).map(Arc::new);
        self.cache_model(key, built.clone());
        built
    }

    /// The stat bytes of the weapon factory `name` makes for an item: its
    /// template bytes changed by the chosen parts and deforms, then scaled by
    /// the item's condition. When the weapon's model has already been built,
    /// the assembled striking extents are included as well.
    pub fn factory_weapon_stats(&self, name: &str, weapon: Option<&Weapon>) -> Option<StatBytes> {
        let key = factory_key(name, weapon);
        if let Some(stats) = self.cache.factory_stats.get(&key) {
            return Some(*stats);
        }
        let (factory, built) = self.factory_recipe(name, weapon)?;
        let (quality, damage) = weapon.map_or((0, 0), |w| (w.quality, w.damage));
        let template = weapon.map_or_else(StatBytes::default, |w| w.stats);
        let stats = weapon_stats::factory_stats(&factory, &built.generated, template);
        Some(weapon_stats::condition(stats, quality, damage))
    }

    /// The factory and the weapon the item's seeds generate.
    fn factory_recipe(&self, name: &str, weapon: Option<&Weapon>) -> Option<(Factory, Built)> {
        let pack = self.game.pack(PackKind::Factories)?;
        let bytes = pack
            .get(name)
            .or_else(|| pack.get(&format!("{name}.fty")))?;
        let factory = match Factory::parse(bytes) {
            Ok(f) => f,
            Err(e) => {
                // The caller reports the bad recipe once per distinct model.
                warn!("factory {name}: {e}");
                return None;
            }
        };
        let spec = weapon.map_or_else(
            || WeaponSpec {
                seeds: [0; 2],
                quality: 0,
                ..Default::default()
            },
            |w| w.spec(0),
        );
        let built = factory.build(&spec);
        Some((factory, built))
    }

    fn build_factory_weapon(
        &mut self,
        name: &str,
        weapon: Option<&Weapon>,
        key: &str,
    ) -> Option<Model> {
        let (factory, built) = self.factory_recipe(name, weapon)?;
        let Generated {
            selection,
            materials: area_materials,
            ..
        } = &built.generated;
        let (quality, damage, dirt) = weapon.map_or((0, 0, 0), |w| (w.quality, w.damage, w.dirt));

        // Load every chosen component and apply its own deforms, then take
        // the box each attach mode reads. Components are cloned per item so
        // deforms never leak into the shared models.
        let mut meshes: Vec<Vec<Mesh>> = vec![Vec::new(); factory.parts.len()];
        let mut boxes = vec![ComponentBox::default(); factory.parts.len()];
        for (i, part) in factory.parts.iter().enumerate() {
            let sel = selection.get(i).copied().unwrap_or(0) as usize;
            if part.flags & 1 != 0 || sel == 0 || sel > part.variants.len() {
                continue;
            }
            let variant = &part.variants[sel - 1];
            let Some(component) = self.model(PackKind::Components, &variant.model) else {
                continue;
            };
            let values = built.generated.deform.get(i).copied().unwrap_or([0.0; 3]);
            let mut loaded = Vec::new();
            for node in &component.nodes {
                for (handle, _) in &node.parts {
                    let Some(mut mesh) = self.meshes.get(handle).cloned() else {
                        continue;
                    };
                    for (d, value) in part.deforms.iter().zip(values) {
                        deform_mesh(&mut mesh, d, value);
                    }
                    loaded.push(mesh);
                }
            }
            boxes[i] = component_box(&loaded);
            meshes[i] = loaded;
        }

        let placed = factory.place(&built.generated, &boxes);
        let mut nodes = vec![ModelNode {
            name: name.to_string(),
            parent: None,
            transform: Transform::IDENTITY,
            parts: Vec::new(),
            part_materials: Vec::new(),
            skin_bone: None,
            light: None,
            voxel_light: None,
            fire: None,
            portal: None,
            bounds: (Vec3::ZERO, 1.0),
            half_extents: Vec3::ZERO,
            gi_points: Default::default(),
            rigid: None,
            tracks: Vec::new(),
            shadow_bias: 0,
            is_skin: false,
        }];
        let mut head: Option<(f32, f32)> = None;
        let mut flipped: Option<(f32, f32)> = None;
        let mut root_top = 0.0f32;
        for placed in &placed {
            let part = &factory.parts[placed.part];
            let variant = &part.variants[placed.variant];
            // The nearest ancestor's deforms marked 0x40000 also shape this
            // part's mesh.
            if let Some(anc) = placed.attach_to {
                let values = built.generated.deform.get(anc).copied().unwrap_or([0.0; 3]);
                let parent_deforms: Vec<_> = factory.parts[anc]
                    .deforms
                    .iter()
                    .zip(values)
                    .filter(|(d, _)| d.flags & 0x40000 != 0)
                    .collect();
                for (d, value) in parent_deforms {
                    for mesh in &mut meshes[placed.part] {
                        deform_mesh(mesh, d, value);
                    }
                }
            }
            let mut layers = [Layer::default(); 8];
            for (k, &area) in variant.mask_areas.iter().enumerate() {
                let Some(area) = area_materials.get(area as usize) else {
                    continue;
                };
                layers[k] = Layer {
                    mask: k as u8 + 1,
                    random_set: 0,
                    material: area.material,
                    colour: area.colour,
                };
            }
            let appearance = Appearance {
                quality,
                damage,
                dirt,
                layers,
            };
            let material = self.painted_factory(&variant.model.to_ascii_lowercase(), &appearance);
            let mut parts = Vec::new();
            for mesh in meshes[placed.part].drain(..) {
                parts.push((self.meshes.add(mesh), material.clone()));
            }
            let [rx, ry, rz] = placed.rotation;
            let transform = Transform {
                translation: Vec3::from_array(placed.position) / CM_PER_METRE,
                rotation: Quat::from_euler(EulerRot::YXZ, ry, rx, rz),
                scale: Vec3::from_array(placed.scale),
            };
            let box_ = boxes[placed.part];
            let bottom = placed.position[1] + box_.centre[1] - box_.half[1];
            let top = placed.position[1] + box_.centre[1] + box_.half[1];
            root_top = root_top.max(top);
            if part.role == 2 || part.role == 3 {
                head = Some((bottom, top));
            } else if part.role == 6 {
                flipped = Some((bottom, top));
            }
            let count = parts.len();
            nodes.push(ModelNode {
                name: variant.model.clone(),
                parent: Some(0),
                transform,
                parts,
                part_materials: vec![String::new(); count],
                skin_bone: None,
                light: None,
                voxel_light: None,
                fire: None,
                portal: None,
                bounds: (Vec3::ZERO, 0.5),
                half_extents: Vec3::ZERO,
                gi_points: Default::default(),
                rigid: None,
                tracks: Vec::new(),
                shadow_bias: 0,
                is_skin: false,
            });
        }

        // The assembled striking extents are a fraction of the weapon's
        // length, in 1/200ths (the item section's start and end bytes).
        if root_top > 0.0 {
            let (quality, damage) = weapon.map_or((0, 0), |w| (w.quality, w.damage));
            let template = weapon.map_or_else(StatBytes::default, |w| w.stats);
            let mut stats = weapon_stats::factory_stats(&factory, &built.generated, template);
            stats = weapon_stats::condition(stats, quality, damage);
            if let Some((bottom, top)) = head {
                stats.set(weapon_stats::at::START, extent(bottom, root_top));
                stats.set(weapon_stats::at::END, extent(top, root_top));
            }
            if let Some((bottom, top)) = flipped {
                stats.set(
                    weapon_stats::at::START + weapon_stats::at::FLIP,
                    extent(bottom, root_top),
                );
                stats.set(
                    weapon_stats::at::END + weapon_stats::at::FLIP,
                    extent(top, root_top),
                );
            }
            self.cache.factory_stats.insert(key.to_string(), stats);
        }
        Some(Model { nodes })
    }
}

/// A factory weapon's model and stat cache key.
fn factory_key(name: &str, weapon: Option<&Weapon>) -> String {
    format!("factory/{}/{:?}", name.to_ascii_lowercase(), weapon)
}

/// A striking extent as the section stores it: a 1/200th of the weapon's
/// length, rounded to nearest.
fn extent(y: f32, length: f32) -> u8 {
    ((y / length) * 200.0).round_ties_even() as i64 as u8
}

/// Applies one deform to a mesh's positions. Meshes are in metres and the
/// recipe in centimetres.
fn deform_mesh(mesh: &mut Mesh, d: &exa_formats::factory::Deform, value: f32) {
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    else {
        return;
    };
    let mut cm: Vec<[f32; 3]> = positions
        .iter()
        .map(|p| {
            [
                p[0] * CM_PER_METRE,
                p[1] * CM_PER_METRE,
                p[2] * CM_PER_METRE,
            ]
        })
        .collect();
    deform_positions(&mut cm, d, value);
    for (p, c) in positions.iter_mut().zip(cm) {
        *p = [
            c[0] / CM_PER_METRE,
            c[1] / CM_PER_METRE,
            c[2] / CM_PER_METRE,
        ];
    }
}

/// The box of a component's meshes after their deforms, centimetres.
fn component_box(meshes: &[Mesh]) -> ComponentBox {
    let (mut min, mut max) = ([f32::MAX; 3], [f32::MIN; 3]);
    let mut any = false;
    for mesh in meshes {
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            continue;
        };
        for p in positions {
            any = true;
            for k in 0..3 {
                min[k] = min[k].min(p[k] * CM_PER_METRE);
                max[k] = max[k].max(p[k] * CM_PER_METRE);
            }
        }
    }
    if !any {
        return ComponentBox::default();
    }
    let centre = std::array::from_fn(|k| (min[k] + max[k]) * 0.5);
    let half = std::array::from_fn(|k| (max[k] - min[k]) * 0.5);
    ComponentBox { centre, half }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ModelCache;
    use bevy::asset::Assets;
    use bevy::image::Image;
    use bevy::prelude::Mesh;
    use exa_render::material::ExaMaterial;

    /// Assembles the first few shipped factories (skipped without the game):
    /// every node must have a finite transform, a mesh and a painted
    /// material, and the stat pass must know the assembled striking extent.
    #[test]
    fn shipped_factory_weapons_assemble() {
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = crate::GameData::open(dir).unwrap();
        let pack = game.pack(PackKind::Factories).unwrap();
        let names: Vec<String> = pack
            .entries()
            .iter()
            .take(6)
            .map(|e| e.name.to_string())
            .collect();
        let mut cache = ModelCache::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<ExaMaterial>::default();
        let mut images = Assets::<Image>::default();
        let mut loader = ModelLoader {
            game: &game,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: false,
        };
        for name in names {
            let model = loader
                .factory_weapon(&name, None)
                .unwrap_or_else(|| panic!("{name}: no model"));
            assert!(model.nodes.len() > 1, "{name}: no parts");
            for node in &model.nodes {
                let t = node.transform;
                assert!(
                    t.translation.is_finite() && t.rotation.is_finite() && t.scale.is_finite(),
                    "{name}: non-finite transform"
                );
            }
            let stats = loader
                .factory_weapon_stats(&name, None)
                .unwrap_or_else(|| panic!("{name}: no stats"));
            let start = stats.get(weapon_stats::at::START);
            let end = stats.get(weapon_stats::at::END);
            assert!(
                start <= end && end <= 200,
                "{name}: striking extent {start}..{end}"
            );
        }
    }
}
