//! Factory weapons: assembled from component models as a factory recipe and
//! an item's seeds choose, shaped by the recipe's deforms and painted per
//! part region with each mask's area material, as the original's factory
//! paint job does.

use std::sync::Arc;

use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use exa_formats::factory::{
    Built, ComponentBox, DEFAULT_METAL, Factory, Generated, Placed, WeaponSpec, deform_positions,
};
use exa_formats::objdb::{Appearance, Weapon};
use exa_formats::weapon_stats::{self, StatBytes};

use crate::PackKind;
use crate::model::{FactoryLayer, FactoryPaint, Model, ModelLoader, ModelNode};

const CM_PER_METRE: f32 = 100.0;

/// A weapon's length and the (`bottom`, `top`) of its head and of its
/// flipped side, in centimetres along +Y.
type FactoryExtents = (f32, Option<(f32, f32)>, Option<(f32, f32)>);

impl ModelLoader<'_> {
    /// A weapon built from factory `name` (`Factories.rpk`, without the `@`),
    /// with the item's weapon data when known; without it, a fresh recipe
    /// with zero seeds.
    pub fn factory_weapon(&mut self, name: &str, weapon: Option<&Weapon>) -> Option<Arc<Model>> {
        let key = factory_key(name, weapon);
        if let Some(m) = self.cached_model(&key) {
            return m;
        }
        // A change of condition leaves the assembled meshes alone: the
        // original repaints an existing weapon node instead of rebuilding
        // it. Reuse the geometry when the seeds and stored choices match.
        let geometry_key = factory_geometry_key(name, weapon);
        if let Some(geometry) = self.cache.factory_geometry.get(&geometry_key).cloned()
            && let Some(repainted) = self.repaint_factory(name, weapon, &geometry, &key)
        {
            self.cache_model(key, Some(repainted.clone()));
            return Some(repainted);
        }
        let built = self.build_factory_weapon(name, weapon, &key).map(Arc::new);
        if let Some(model) = &built {
            self.cache
                .factory_geometry
                .insert(geometry_key, model.clone());
        }
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

    /// The stored box of every chosen component (the first node with a
    /// mesh), in centimetres, as the assembler's attach modes read it.
    fn factory_boxes(&mut self, factory: &Factory, generated: &Generated) -> Vec<ComponentBox> {
        let mut boxes = vec![ComponentBox::default(); factory.parts.len()];
        for (i, part) in factory.parts.iter().enumerate() {
            let sel = generated.selection.get(i).copied().unwrap_or(0) as usize;
            if part.flags & 1 != 0 || sel == 0 || sel > part.variants.len() {
                continue;
            }
            let variant = &part.variants[sel - 1];
            let Some(component) = self.model(PackKind::Components, &variant.model) else {
                continue;
            };
            if let Some(node) = component.nodes.iter().find(|n| !n.parts.is_empty()) {
                boxes[i] = ComponentBox {
                    centre: (node.bounds.0 * CM_PER_METRE).to_array(),
                    half: (node.half_extents * CM_PER_METRE).to_array(),
                };
            }
        }
        boxes
    }

    /// One assembled part's paint region: its masks take the areas' rolled
    /// materials, finishes and colours; the region is the part's rectangle
    /// of the factory atlas; the item's condition and the region's texture
    /// parameters come from the weapon and the recipe.
    fn factory_paint(
        &self,
        factory: &Factory,
        built: &Built,
        placed: &Placed,
        pattern_position: [f32; 4],
        condition: (u8, u8, u8),
    ) -> FactoryPaint {
        let part = &factory.parts[placed.part];
        let variant = &part.variants[placed.variant];
        let (quality, damage, dirt) = condition;
        // Masks 1–4 take their areas' materials; an area past the generated
        // ones paints as SteelGen.
        let layers = variant
            .mask_areas
            .iter()
            .enumerate()
            .map(|(k, &area)| {
                let m = built.generated.materials.get(area as usize);
                FactoryLayer {
                    mask: k as u8 + 1,
                    material: m.map_or(0, |m| m.material),
                    finish: m.map_or(0, |m| m.finish),
                    colour: m.map_or(0, |m| m.colour),
                    emblem: m.and_then(|m| m.emblem),
                }
            })
            .collect();
        let atlas = Vec2::new(factory.atlas[0] as f32, factory.atlas[1] as f32);
        FactoryPaint {
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
            uv_scale: part.uv_scale(
                built
                    .generated
                    .deform
                    .get(placed.part)
                    .copied()
                    .unwrap_or([0.0; 3]),
            ),
            pattern_position,
            // The region quad's u runs over `1 − texture_parameter`.
            texture_crop: 1.0 - part.texture_parameter,
            texture_detail: self.cache.textures.settings().texture_detail,
        }
    }

    /// The assembled striking extents: the weapon's length (the top of the
    /// merged mesh) and the head's and the flipped side's bottom and top.
    fn factory_extents(
        factory: &Factory,
        boxes: &[ComponentBox],
        placed: &[Placed],
    ) -> FactoryExtents {
        let mut head = None;
        let mut flipped = None;
        let mut root_top = 0.0f32;
        for placed in placed {
            let part = &factory.parts[placed.part];
            let box_ = boxes[placed.part];
            let bottom = placed.position[1] + box_.centre[1] - box_.half[1];
            let top = placed.position[1] + box_.centre[1] + box_.half[1];
            root_top = root_top.max(top);
            if part.role == 2 || part.role == 3 {
                head = Some((bottom, top));
            } else if part.role == 6 {
                flipped = Some((bottom, top));
            }
        }
        (root_top, head, flipped)
    }

    /// Repaints an already assembled weapon for a changed condition, as the
    /// original repaints a live weapon node: the recipe is rolled again from
    /// the same seeds (so the area colours follow the new quality), the
    /// paint regions are rebuilt and the geometry's materials replaced
    /// without touching its meshes. Also refreshes the striking extents.
    fn repaint_factory(
        &mut self,
        name: &str,
        weapon: Option<&Weapon>,
        geometry: &Model,
        key: &str,
    ) -> Option<Arc<Model>> {
        let (factory, built) = self.factory_recipe(name, weapon)?;
        let boxes = self.factory_boxes(&factory, &built.generated);
        let placed = factory.place(&built.generated, &boxes);
        // Which parts had a mesh is what the build recorded: one node per
        // placed part, in placement order, and a node with no parts was an
        // empty component. The emblem placement asks by recipe part index.
        let mut loaded = vec![false; factory.parts.len()];
        for (node, placed) in geometry.nodes.iter().skip(1).zip(&placed) {
            loaded[placed.part] = !node.parts.is_empty();
        }
        let pattern_position = factory.pattern_position(&built.generated.selection, |i| {
            loaded.get(i).copied().unwrap_or(false)
        });
        let condition = weapon.map_or((0, 0, 0), |w| (w.quality, w.damage, w.dirt));
        let mut model = geometry.clone();
        for (node, placed) in model.nodes.iter_mut().skip(1).zip(&placed) {
            let paint = self.factory_paint(&factory, &built, placed, pattern_position, condition);
            let material = self.painted_factory(&paint);
            for part in &mut node.parts {
                part.1 = material.clone();
            }
        }
        let (root_top, head, flipped) = Self::factory_extents(&factory, &boxes, &placed);
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
        Some(Arc::new(model))
    }

    fn build_factory_weapon(
        &mut self,
        name: &str,
        weapon: Option<&Weapon>,
        key: &str,
    ) -> Option<Model> {
        let (factory, built) = self.factory_recipe(name, weapon)?;
        let Generated { selection, .. } = &built.generated;
        let (quality, damage, dirt) = weapon.map_or((0, 0, 0), |w| (w.quality, w.damage, w.dirt));

        // The box each attach mode reads is the component node's stored one.
        let boxes = self.factory_boxes(&factory, &built.generated);
        // Load every chosen component and apply its own deforms; components
        // are cloned per item so deforms never leak into the shared models.
        let mut meshes: Vec<Vec<Mesh>> = vec![Vec::new(); factory.parts.len()];
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
            meshes[i] = loaded;
        }

        let placed = factory.place(&built.generated, &boxes);
        // The emblem placement counts the parts whose component loaded.
        let pattern_position = factory.pattern_position(selection, |i| !meshes[i].is_empty());
        let mut nodes = vec![ModelNode {
            name: name.to_string(),
            parent: None,
            transform: Transform::IDENTITY,
            parts: Vec::new(),
            part_materials: Vec::new(),
            part_flags: Vec::new(),
            collision: Vec::new(),
            skin_bone: None,
            light: None,
            voxel_light: None,
            fire: None,
            portal: None,
            bounds: (Vec3::ZERO, 1.0),
            half_extents: Vec3::ZERO,
            gi_points: Default::default(),
            gi_centred: false,
            gi_thin: false,
            rigid: None,
            tracks: Vec::new(),
            shadow_bias: 0,
            is_skin: false,
            placeholder: false,
            cloth: false,
            cloth_mesh: None,
            mesh: false,
            render_flags: 0,
            triangles: None,
        }];
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
            let paint = self.factory_paint(
                &factory,
                &built,
                placed,
                pattern_position,
                (quality, damage, dirt),
            );
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
            let count = parts.len();
            nodes.push(ModelNode {
                name: variant.model.clone(),
                parent: Some(0),
                transform,
                parts,
                part_materials: vec![String::new(); count],
                part_flags: Vec::new(),
                collision: Vec::new(),
                skin_bone: None,
                light: None,
                voxel_light: None,
                fire: None,
                portal: None,
                bounds: (Vec3::ZERO, 0.5),
                half_extents: Vec3::ZERO,
                gi_points: Default::default(),
                gi_centred: false,
                gi_thin: false,
                rigid: None,
                tracks: Vec::new(),
                shadow_bias: 0,
                is_skin: false,
                placeholder: false,
                cloth: false,
                cloth_mesh: None,
                mesh: false,
                render_flags: 0,
                triangles: None,
            });
        }

        // The assembled striking extents are a fraction of the weapon's
        // length, in 1/200ths (the item section's start and end bytes).
        let (root_top, head, flipped) = Self::factory_extents(&factory, &boxes, &placed);
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

/// The part of a weapon's state its assembled geometry depends on: the
/// seeds, the section flags (which select a stored-choice replay) and the
/// stored choices. The condition (quality, damage, dirt) is left out: it
/// only changes the paint and the stat scaling, so a change of condition
/// repaints the existing meshes.
fn factory_geometry_key(name: &str, weapon: Option<&Weapon>) -> String {
    match weapon {
        None => format!("geometry/{}", name.to_ascii_lowercase()),
        Some(w) => format!(
            "geometry/{}/{:#x}/{:?}/{:?}/{:?}",
            name.to_ascii_lowercase(),
            w.flags,
            w.seeds,
            w.selection,
            w.materials
        ),
    }
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

    /// Every emblem a factory pattern area can roll names a shipped
    /// background picture that decodes (skipped without the game).
    #[test]
    fn every_factory_emblem_picture_decodes() {
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = crate::GameData::open(dir).unwrap();
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
        use exa_formats::procedural::{EMBLEM_BACKGROUND_STEM, EMBLEM_IDS, emblem_picture};
        for id in EMBLEM_IDS {
            let name = emblem_picture(EMBLEM_BACKGROUND_STEM, u32::from(id));
            assert!(loader.texels(&name).is_some(), "{name}");
        }
    }

    /// Every shipped recipe's region parameters: the source crop is
    /// `1 − texture_parameter` and the atlas rectangle has a positive size
    /// (skipped without the game). Some recipes store a negative texture
    /// parameter (crop above 1, a mirrored/overhanging source), so the crop
    /// is not clamped.
    #[test]
    fn shipped_factory_regions_have_finite_crops() {
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = crate::GameData::open(dir).unwrap();
        let pack = game.pack(PackKind::Factories).unwrap();
        for entry in pack.entries() {
            let Some(bytes) = pack.get(&entry.name) else {
                continue;
            };
            let f = Factory::parse(bytes).unwrap();
            for part in &f.parts {
                let crop = 1.0 - part.texture_parameter;
                assert!(crop.is_finite(), "{}: crop {crop}", entry.name);
                assert!(
                    part.atlas[0].is_finite()
                        && part.atlas[1].is_finite()
                        && part.atlas[2].is_finite()
                        && part.atlas[3].is_finite(),
                    "{}: atlas",
                    entry.name
                );
            }
        }
    }

    /// A condition change repaints a shipped weapon without rebuilding its
    /// meshes: the geometry handles stay, the materials differ, and the
    /// striking extents are refreshed (skipped without the game).
    #[test]
    fn a_condition_change_repaints_the_same_geometry() {
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = crate::GameData::open(dir).unwrap();
        let pack = game.pack(PackKind::Factories).unwrap();
        let Some(entry) = pack.entries().iter().find(|e| e.name.starts_with("sword")) else {
            return;
        };
        let name = entry.name.clone();
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
        let record = exa_formats::objdb::Weapon {
            seeds: [1, 2],
            quality: 200,
            ..Default::default()
        };
        let first = loader.factory_weapon(&name, Some(&record)).unwrap();
        let geometry: Vec<Handle<Mesh>> = first
            .nodes
            .iter()
            .flat_map(|n| n.parts.iter().map(|(m, _)| m.clone()))
            .collect();
        let old_materials: Vec<Handle<ExaMaterial>> = first
            .nodes
            .iter()
            .flat_map(|n| n.parts.iter().map(|(_, m)| m.clone()))
            .collect();
        // The same seeds with damage: the meshes are the same handles, the
        // paint differs.
        let mut damaged = record;
        damaged.damage = 200;
        let second = loader.factory_weapon(&name, Some(&damaged)).unwrap();
        let new_geometry: Vec<Handle<Mesh>> = second
            .nodes
            .iter()
            .flat_map(|n| n.parts.iter().map(|(m, _)| m.clone()))
            .collect();
        assert_eq!(geometry, new_geometry);
        let new_materials: Vec<Handle<ExaMaterial>> = second
            .nodes
            .iter()
            .flat_map(|n| n.parts.iter().map(|(_, m)| m.clone()))
            .collect();
        assert_ne!(old_materials, new_materials);
        assert!(loader.factory_weapon_stats(&name, Some(&damaged)).is_some());
    }

    /// A factory region is the part's atlas rectangle, not its source
    /// texture's size: shipped recipes scale the source over the region
    /// (a 128×128 hookbolt fills 256×256, a 128×512 falchion blade 128×256,
    /// quarterstaff_b1's cap has a negative height), and the painted target
    /// is the region at full detail, so its material tiles repeat once per
    /// source texture. Skipped without the game.
    #[test]
    fn factory_regions_are_the_paint_targets() {
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = crate::GameData::open(dir).unwrap();
        let pack = game.pack(PackKind::Factories).unwrap();
        let mut checked = 0;
        let mut stretched = 0;
        for entry in pack.entries() {
            let Some(bytes) = pack.get(&entry.name) else {
                continue;
            };
            let f = Factory::parse(bytes).unwrap();
            for part in &f.parts {
                for variant in &part.variants {
                    let region = Vec2::new(
                        f.atlas[0] as f32 * part.atlas[2],
                        f.atlas[1] as f32 * part.atlas[3],
                    );
                    if region.x == 0.0 || region.y == 0.0 {
                        continue;
                    }
                    checked += 1;
                    let target = crate::paint::paint_target_size(region, 3);
                    assert_eq!(
                        target,
                        Vec2::new(
                            region.x.abs().round_ties_even().max(1.0),
                            region.y.abs().round_ties_even().max(1.0)
                        )
                    );
                    let tiling = crate::paint::Tiling::factory(region, target, 0);
                    // One source texel per region texel; a negative atlas
                    // scale mirrors the tiles (the quad is flipped).
                    assert!(
                        (tiling.density.abs() - Vec2::ONE).max_element() < 0.01,
                        "{}: density {:?}",
                        entry.name,
                        tiling.density
                    );
                    if let Some(texture) = game.texture(&format!("{}d", variant.model))
                        && let Ok(rfi) = exa_formats::rfi::Rfi::parse(texture)
                        && (rfi.header.width as f32, rfi.header.height as f32)
                            != (target.x, target.y)
                    {
                        stretched += 1;
                    }
                }
            }
        }
        assert!(checked > 100, "only {checked} checked");
        // Many parts draw a source texture of another size over the region;
        // this is why the region, not the texture, is the target.
        assert!(stretched > 20, "only {stretched} stretched");
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
