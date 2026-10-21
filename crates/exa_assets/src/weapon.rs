//! Factory weapons: assembled from component models as a factory recipe and
//! an item's seeds choose, shaped by the recipe's deforms and painted per
//! material area like apparel.

use std::sync::Arc;

use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use exa_formats::factory::{Deform, Factory, band_stretch, deform_factor};
use exa_formats::objdb::{Appearance, Layer, Weapon};
use exa_formats::weapon_stats::{self, StatBytes};

use crate::PackKind;
use crate::model::{Model, ModelLoader, ModelNode};

const CM_PER_METRE: f32 = 100.0;
/// Palette colour of areas whose colour is decided at run time.
const DEFAULT_COLOUR: u8 = 3;

impl ModelLoader<'_> {
    /// A weapon built from factory `name` (`Factories.rpk`, without the `@`),
    /// with the item's weapon data when known; without it, a fixed seed.
    pub fn factory_weapon(&mut self, name: &str, weapon: Option<&Weapon>) -> Option<Arc<Model>> {
        let key = format!("factory/{}/{:?}", name.to_ascii_lowercase(), weapon);
        if let Some(m) = self.cached_model(&key) {
            return m;
        }
        let built = self.build_factory_weapon(name, weapon).map(Arc::new);
        self.cache_model(key, built.clone());
        built
    }

    /// The stat bytes of the weapon factory `name` makes for an item: its
    /// template bytes changed by the chosen parts and deforms, then scaled by
    /// the item's condition.
    pub fn factory_weapon_stats(&self, name: &str, weapon: Option<&Weapon>) -> Option<StatBytes> {
        let pack = self.game.pack(PackKind::Factories)?;
        let bytes = pack
            .get(name)
            .or_else(|| pack.get(&format!("{name}.fty")))?;
        let factory = Factory::parse(bytes).ok()?;
        let (seeds, quality, damage) = weapon.map_or(([0x1234_5678; 2], 128, 0), |w| {
            (w.seeds, w.quality, w.damage)
        });
        let stored = weapon
            .filter(|w| w.has_stored_choices())
            .map(|w| w.selection.as_slice());
        let generated = factory.generate(seeds[0], seeds[1], quality, stored);
        let template = weapon.map_or_else(StatBytes::default, |w| w.stats);
        let stats = weapon_stats::factory_stats(&factory, &generated, template);
        Some(weapon_stats::condition(stats, quality, damage))
    }

    fn build_factory_weapon(&mut self, name: &str, weapon: Option<&Weapon>) -> Option<Model> {
        let pack = self.game.pack(PackKind::Factories)?;
        let bytes = pack
            .get(name)
            .or_else(|| pack.get(&format!("{name}.fty")))?;
        let factory = match Factory::parse(bytes) {
            Ok(f) => f,
            Err(e) => {
                self.warn_once(format!("factory {name}: {e}"));
                return None;
            }
        };
        let (seeds, quality) = weapon.map_or(([0x1234_5678; 2], 128), |w| (w.seeds, w.quality));
        let stored = weapon
            .filter(|w| w.has_stored_choices())
            .map(|w| w.selection.as_slice());
        let generated = factory.generate(seeds[0], seeds[1], quality, stored);
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
        for placed in factory.place(&generated) {
            let part = &factory.parts[placed.part];
            let variant = &part.variants[placed.variant];
            let Some(component) = self.model(PackKind::Components, &variant.model) else {
                continue;
            };
            // Materials of the component's masks, from its areas.
            let mut layers = [Layer::default(); 8];
            for (k, &area) in variant.mask_areas.iter().enumerate() {
                let area = area as usize;
                let from_item = weapon
                    .filter(|w| w.has_stored_choices() && area < 4)
                    .map(|w| w.materials[area])
                    .filter(|m| m.0 != 0);
                let (material, colour) = match (from_item, generated.materials.get(area)) {
                    (Some((m, _, c)), _) => (m, c),
                    (None, Some(g)) => (g.material, g.colour.unwrap_or(DEFAULT_COLOUR)),
                    _ => continue,
                };
                layers[k] = Layer {
                    mask: k as u8 + 1,
                    random_set: 0,
                    material,
                    colour,
                };
            }
            let appearance = Appearance {
                quality,
                damage: weapon.map_or(0, |w| w.damage),
                dirt: weapon.map_or(0, |w| w.dirt),
                layers,
            };
            let material = self.painted_apparel(&variant.model.to_ascii_lowercase(), &appearance);
            let deform_values = generated.deform[placed.part];
            let mut parts = Vec::new();
            for node in &component.nodes {
                for (mesh, _) in &node.parts {
                    let Some(mut mesh) = self.meshes.get(mesh).cloned() else {
                        continue;
                    };
                    deform_mesh(&mut mesh, &part.deforms, &deform_values);
                    parts.push((self.meshes.add(mesh), material.clone()));
                }
            }
            let [rx, ry, rz] = placed.rotation;
            let transform = Transform {
                translation: Vec3::from_array(placed.position) / CM_PER_METRE,
                rotation: Quat::from_euler(EulerRot::YXZ, ry, rx, rz),
                scale: Vec3::from_array(placed.scale),
            };
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
        Some(Model { nodes })
    }
}

/// Applies a part's scale (kind 1) and band-stretch (kind 2) deforms to its
/// vertices. Tapers, bends and radial deforms are not reproduced yet.
fn deform_mesh(mesh: &mut Mesh, deforms: &[Deform; 3], values: &[f32; 3]) {
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    else {
        return;
    };
    for (d, &v) in deforms.iter().zip(values) {
        let f = deform_factor(v, d.amount);
        match d.kind {
            1 => {
                let pivot = d.pivot / CM_PER_METRE;
                let axes: &[usize] = match d.axis {
                    0 => &[0, 1, 2],
                    1 => &[0],
                    2 => &[1],
                    3 => &[2],
                    4 => &[0, 1],
                    5 => &[0, 2],
                    6 => &[1, 2],
                    _ => &[],
                };
                for p in positions.iter_mut() {
                    for &k in axes {
                        // Only the long axis carries a pivot; the others
                        // scale about the part's axis.
                        let centre = if k == 1 { pivot } else { 0.0 };
                        p[k] = (p[k] - centre) * f + centre;
                    }
                }
            }
            2 => {
                let band = Deform {
                    pivot: d.pivot / CM_PER_METRE,
                    range: d.range / CM_PER_METRE,
                    ..*d
                };
                for p in positions.iter_mut() {
                    p[1] = band_stretch(p[1], &band, f);
                }
            }
            _ => {}
        }
    }
}
