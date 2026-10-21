//! Factory weapons: assembled from component models as a factory recipe and
//! an item's seeds choose, shaped by the recipe's deforms and painted per
//! part region with each mask's area material, as the original's factory
//! paint job does.

use std::sync::Arc;

use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use exa_formats::factory::{
    Built, ComponentBox, DEFAULT_METAL, Factory, Generated, WeaponSpec, deform_positions,
};
use exa_formats::objdb::{Appearance, Weapon};
use exa_formats::weapon_stats::{self, StatBytes};

use crate::PackKind;
use crate::model::{FactoryLayer, FactoryPaint, Model, ModelLoader, ModelNode};

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
    pub fn factory_weapon_stats(
        &mut self,
        name: &str,
        weapon: Option<&Weapon>,
    ) -> Option<StatBytes> {
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

    /// The factory and the weapon the item's seeds generate. Parsed recipes
    /// are kept, as the original's factory cache does.
    fn factory_recipe(&mut self, name: &str, weapon: Option<&Weapon>) -> Option<(Factory, Built)> {
        let key = name.to_ascii_lowercase();
        let factory = if let Some(f) = self.cache.factories.get(&key) {
            f.clone()
        } else {
            let pack = self.game.pack(PackKind::Factories)?;
            let bytes = pack
                .get(name)
                .or_else(|| pack.get(&format!("{name}.fty")))?;
            let parsed = match Factory::parse(bytes) {
                Ok(f) => f,
                Err(e) => {
                    // The caller reports the bad recipe once per distinct model.
                    warn!("factory {name}: {e}");
                    return None;
                }
            };
            let f = Arc::new(parsed);
            self.cache.factories.insert(key, f.clone());
            f
        };
        let spec = weapon.map_or_else(WeaponSpec::default, |w| w.spec(DEFAULT_METAL));
        let built = factory.build(&spec);
        Some(((*factory).clone(), built))
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

        // Load every chosen component and apply its own deforms; the box each
        // attach mode reads is the component node's stored one. Components
        // are cloned per item so deforms never leak into the shared models.
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
            if let Some(node) = component.nodes.iter().find(|n| !n.parts.is_empty()) {
                boxes[i] = ComponentBox {
                    centre: (node.bounds.0 * CM_PER_METRE).to_array(),
                    half: (node.half_extents * CM_PER_METRE).to_array(),
                };
            }
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
            cull_front: false,
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
            // Masks 1–4 take their areas' materials; an area past the
            // generated ones paints as SteelGen.
            let layers = variant
                .mask_areas
                .iter()
                .enumerate()
                .map(|(k, &area)| {
                    let m = area_materials.get(area as usize);
                    FactoryLayer {
                        mask: k as u8 + 1,
                        material: m.map_or(0, |m| m.material),
                        finish: m.map_or(0, |m| m.finish),
                        colour: m.map_or(0, |m| m.colour),
                    }
                })
                .collect();
            let atlas = Vec2::new(factory.atlas[0] as f32, factory.atlas[1] as f32);
            let paint = FactoryPaint {
                stem: variant.model.clone(),
                condition: Appearance {
                    quality,
                    damage,
                    dirt,
                    layers: Default::default(),
                },
                layers,
                region_texels: atlas * Vec2::new(part.atlas[2], part.atlas[3]),
                seed_b: built.seeds[1],
            };
            let material = self.painted_factory(&paint);
            let mut parts = Vec::new();
            for mesh in meshes[placed.part].drain(..) {
                parts.push((self.meshes.add(mesh), material.clone()));
            }
            let [rx, ry, rz] = placed.rotation;
            let transform = Transform {
                translation: Vec3::from_array(placed.position) / CM_PER_METRE,
                // The engine's Euler order: X, then Z, then Y.
                rotation: Quat::from_euler(EulerRot::YZX, ry, rz, rx),
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
                cull_front: false,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ModelCache;
    use bevy::asset::Assets;
    use bevy::image::Image;
    use bevy::prelude::Mesh;
    use exa_render::material::ExaMaterial;

    #[test]
    fn the_striking_extent_is_two_hundredths_a_length() {
        assert_eq!(extent(0.0, 100.0), 0);
        assert_eq!(extent(50.0, 100.0), 100);
        assert_eq!(extent(75.0, 100.0), 150);
    }

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
        let names: Vec<String> = pack.entries().iter().map(|e| e.name.to_string()).collect();
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
            assert!(!model.nodes.is_empty(), "{name}: no nodes");
            for node in &model.nodes {
                let t = node.transform;
                assert!(
                    t.translation.is_finite() && t.rotation.is_finite() && t.scale.is_finite(),
                    "{name}: non-finite transform"
                );
                for (mesh, material) in &node.parts {
                    assert!(loader.meshes.get(mesh).is_some(), "{name}: missing mesh");
                    assert!(
                        loader.materials.get(material).is_some(),
                        "{name}: missing painted material"
                    );
                }
            }
            let stats = loader
                .factory_weapon_stats(&name, None)
                .unwrap_or_else(|| panic!("{name}: no stats"));
            // A head's start can be a negative fraction of the weapon's
            // length (the baton's head reaches below its root); the section
            // stores it as a wrapped byte, as the original does.
            assert!(stats.get(weapon_stats::at::END) <= 200, "{name}");
        }
    }
}
