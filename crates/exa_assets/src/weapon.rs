//! Factory weapons: assembled from component models as a factory recipe and
//! an item's seeds choose, shaped by the recipe's deforms, merged into one
//! mesh and painted into one shared atlas, each part's region with each
//! mask's area material, as the original's factory and its paint job do.

use std::sync::Arc;

use bevy::prelude::*;
use exa_formats::factory::{
    Built, ComponentBox, DEFAULT_METAL, Factory, Generated, Placed, WeaponSpec, deform_positions,
};
use exa_formats::morph::Frame;
use exa_formats::objdb::{Appearance, Weapon};
use exa_formats::procedural;
use exa_formats::rfc::{Mesh as RfcMesh, NodeBody};
use exa_formats::weapon_stats::{self, StatBytes};
use exa_render::material::ExaMaterial;

use crate::PackKind;
use crate::merge::{merge_editable, regroup_by_material};
use crate::mesh_edit::{self, EditMesh, StreamData};
use crate::model::{FactoryLayer, FactoryPaint, Model, ModelLoader, ModelNode};

const CM_PER_METRE: f32 = 100.0;

/// The (`bottom`, `top`) of a weapon's head and of its flipped side, in
/// centimetres along +Y.
type FactoryExtents = (Option<(f32, f32)>, Option<(f32, f32)>);

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

    /// The stored box of every chosen component (its first mesh node's), in
    /// centimetres, as the assembler's attach modes read it.
    fn factory_boxes(&mut self, factory: &Factory, generated: &Generated) -> Vec<ComponentBox> {
        let mut boxes = vec![ComponentBox::default(); factory.parts.len()];
        for (i, part) in factory.parts.iter().enumerate() {
            let sel = generated.selection.get(i).copied().unwrap_or(0) as usize;
            if part.flags & 1 != 0 || sel == 0 || sel > part.variants.len() {
                continue;
            }
            let variant = &part.variants[sel - 1];
            if let Some(component) = self.factory_component(&variant.model) {
                boxes[i] = component.stored_box;
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

    /// The assembled head's and flipped side's bottom and top: each part's
    /// stored box over its placed position.
    fn factory_extents(
        factory: &Factory,
        boxes: &[ComponentBox],
        placed: &[Placed],
    ) -> FactoryExtents {
        let mut head = None;
        let mut flipped = None;
        for placed in placed {
            let part = &factory.parts[placed.part];
            let box_ = boxes[placed.part];
            let bottom = placed.position[1] + box_.centre[1] - box_.half[1];
            let top = placed.position[1] + box_.centre[1] + box_.half[1];
            if part.role == 2 || part.role == 3 {
                head = Some((bottom, top));
            } else if part.role == 6 {
                flipped = Some((bottom, top));
            }
        }
        (head, flipped)
    }

    /// Repaints an already assembled weapon for a changed condition, as the
    /// original repaints a live weapon node: the recipe is rolled again from
    /// the same seeds (so the area colours follow the new quality), the
    /// atlas is painted again and the merged mesh's material replaced
    /// without touching its mesh. Also refreshes the striking extents.
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
        let mut loaded = vec![false; factory.parts.len()];
        for placed in &placed {
            let model = &factory.parts[placed.part].variants[placed.variant].model;
            loaded[placed.part] = self.factory_component(model).is_some();
        }
        let condition = weapon.map_or((0, 0, 0), |w| (w.quality, w.damage, w.dirt));
        let material = self.factory_material(name, &factory, &built, &placed, &loaded, condition);
        let mut model = geometry.clone();
        for node in &mut model.nodes {
            for part in &mut node.parts {
                part.1 = material.clone();
            }
        }
        let root = model.nodes.first();
        let top = root.map_or(0.0, |n| (n.bounds.0.y + n.half_extents.y) * CM_PER_METRE);
        self.store_factory_extents(&factory, &built, weapon, &boxes, &placed, top, key);
        Some(Arc::new(model))
    }

    /// A component's first node as loaded, its groups' split flags (from
    /// their materials) and its stored box, cached. `None` when the
    /// component is missing or its first node is not a mesh.
    fn factory_component(&mut self, model: &str) -> Option<Arc<FactoryComponent>> {
        let key = model.to_ascii_lowercase();
        if let Some(c) = self.cache.components.get(&key) {
            return c.clone();
        }
        let component = self.model_job(PackKind::Components, model).and_then(|job| {
            let flags = self.split_flags(&job.scene);
            // The assembler takes the file's first node, and only when it
            // is a mesh.
            job.scene
                .nodes
                .iter()
                .zip(flags)
                .take(1)
                .find_map(|(node, flags)| match &node.body {
                    NodeBody::Mesh(mesh) => Some(Arc::new(FactoryComponent {
                        mesh: mesh.as_ref().clone(),
                        split_flags: flags,
                        stored_box: ComponentBox {
                            centre: node.bounds_centre.map(|c| c * CM_PER_METRE),
                            half: node.bounds_half_extents.map(|c| c * CM_PER_METRE),
                        },
                    })),
                    _ => None,
                })
        });
        self.cache.components.insert(key, component.clone());
        component
    }

    /// The weapon's one material: every assembled part's region painted
    /// into the shared atlas ([`ModelLoader::factory_atlas_material`]).
    fn factory_material(
        &mut self,
        name: &str,
        factory: &Factory,
        built: &Built,
        placed: &[Placed],
        loaded: &[bool],
        condition: (u8, u8, u8),
    ) -> Handle<ExaMaterial> {
        // The emblem placement counts the parts whose component loaded.
        let pattern_position = factory.pattern_position(&built.generated.selection, |i| {
            loaded.get(i).copied().unwrap_or(false)
        });
        let regions: Vec<_> = placed
            .iter()
            .filter(|p| loaded.get(p.part).copied().unwrap_or(false))
            .map(|p| {
                let paint = self.factory_paint(factory, built, p, pattern_position, condition);
                (paint, factory.parts[p.part].atlas)
            })
            .collect();
        let size =
            crate::atlas::job_size(factory.atlas, self.cache.textures.settings().texture_detail);
        self.factory_atlas_material(name, size, &regions)
    }

    /// Writes the assembled striking extents into the weapon's stat bytes:
    /// each a fraction of `top`, the top of the merged mesh's box (cm), in
    /// 1/200ths.
    #[allow(clippy::too_many_arguments)]
    fn store_factory_extents(
        &mut self,
        factory: &Factory,
        built: &Built,
        weapon: Option<&Weapon>,
        boxes: &[ComponentBox],
        placed: &[Placed],
        top: f32,
        key: &str,
    ) {
        let (head, flipped) = Self::factory_extents(factory, boxes, placed);
        if top > 0.0 {
            let (quality, damage) = weapon.map_or((0, 0), |w| (w.quality, w.damage));
            let template = weapon.map_or_else(StatBytes::default, |w| w.stats);
            let mut stats = weapon_stats::factory_stats(factory, &built.generated, template);
            stats = weapon_stats::condition(stats, quality, damage);
            if let Some((bottom, head_top)) = head {
                stats.set(weapon_stats::at::START, extent(bottom, top));
                stats.set(weapon_stats::at::END, extent(head_top, top));
            }
            if let Some((bottom, flipped_top)) = flipped {
                stats.set(
                    weapon_stats::at::START + weapon_stats::at::FLIP,
                    extent(bottom, top),
                );
                stats.set(
                    weapon_stats::at::END + weapon_stats::at::FLIP,
                    extent(flipped_top, top),
                );
            }
            self.cache.factory_stats.insert(key.to_string(), stats);
        }
    }

    /// Assembles a factory weapon as the original does: every chosen part's
    /// component mesh, shaped by its own deforms (and an ancestor's marked
    /// `0x40000`), its texture coordinates taken into the part's atlas
    /// rectangle, is appended at the part's frame to one new mesh; the faces are then regrouped by material, duplicate
    /// elements welded, the mesh converted to render form (its normals
    /// rebuilt from the assembled shape) and its bounds refitted. Every
    /// group draws with the weapon's one atlas material.
    fn build_factory_weapon(
        &mut self,
        name: &str,
        weapon: Option<&Weapon>,
        key: &str,
    ) -> Option<Model> {
        let (factory, built) = self.factory_recipe(name, weapon)?;
        let condition = weapon.map_or((0, 0, 0), |w| (w.quality, w.damage, w.dirt));

        // The box each attach mode reads is the component node's stored one.
        let boxes = self.factory_boxes(&factory, &built.generated);
        let placed = factory.place(&built.generated, &boxes);
        let mut merged = EditMesh::empty();
        let mut loaded = vec![false; factory.parts.len()];
        let mut split = Vec::<(String, u32)>::new();
        let mut appended = 0usize;
        for placed in &placed {
            let part = &factory.parts[placed.part];
            let variant = &part.variants[placed.variant];
            let Some(component) = self.factory_component(&variant.model) else {
                continue;
            };
            loaded[placed.part] = true;
            let FactoryComponent {
                mesh,
                split_flags: flags,
                ..
            } = component.as_ref();
            // Components are copied per item so deforms never leak into the
            // shared mesh. The part's own deforms first, then those of its
            // nearest chosen ancestor marked 0x40000.
            let mut mesh = mesh.clone();
            let values = built
                .generated
                .deform
                .get(placed.part)
                .copied()
                .unwrap_or([0.0; 3]);
            for (d, value) in part.deforms.iter().zip(values) {
                deform_rfc(&mut mesh, d, value);
            }
            if let Some(anc) = placed.attach_to {
                let values = built.generated.deform.get(anc).copied().unwrap_or([0.0; 3]);
                for (d, value) in factory.parts[anc].deforms.iter().zip(values) {
                    if d.flags & 0x40000 != 0 {
                        deform_rfc(&mut mesh, d, value);
                    }
                }
            }
            for (g, group) in mesh.groups.iter().enumerate() {
                let f = flags.get(g).copied().unwrap_or(0);
                if !split.iter().any(|(m, _)| *m == group.material) {
                    split.push((group.material.clone(), f));
                }
            }
            let mut edit = match EditMesh::from_mesh(&mesh, flags) {
                Ok(e) => e,
                Err(e) => {
                    self.warn_once(format!("factory {name}: component {}: {e}", variant.model));
                    continue;
                }
            };
            // The first texture-coordinate stream is moved into the part's
            // atlas rectangle.
            if let Some(stream) = edit
                .streams
                .iter_mut()
                .find(|s| s.format & mesh_edit::KEY_MASK == mesh_edit::KEY_UVS)
                && let StreamData::Vec2(uvs) = &mut stream.data
            {
                for uv in uvs.iter_mut() {
                    *uv = crate::atlas::atlas_uv(*uv, part.atlas);
                }
            }
            if let Err(e) = merge_editable(&mut merged, &edit, part_frame(placed), IDENTITY) {
                self.warn_once(format!("factory {name}: component {}: {e}", variant.model));
                continue;
            }
            appended += 1;
        }
        let rendered = (appended > 0)
            .then(|| {
                regroup_by_material(&mut merged)?;
                // The split flags come from the components' materials: the
                // original converts the mesh before it assigns the weapon's
                // material.
                for group in &mut merged.groups {
                    group.flags = split
                        .iter()
                        .find(|(m, _)| *m == group.material)
                        .map_or(0, |(_, f)| *f);
                }
                merged.weld()?;
                merged.render(true)
            })
            .transpose();
        let rendered = match rendered {
            Ok(r) => r,
            Err(e) => {
                self.warn_once(format!("factory {name}: merged mesh: {e}"));
                None
            }
        };

        let material = self.factory_material(name, &factory, &built, &placed, &loaded, condition);
        let mut root = ModelNode {
            prop_skin: None,
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
            first_part: None,
            floor: false,
            gi_thin: false,
            rigid: None,
            tracks: Vec::new(),
            shadow_bias: 0,
            is_skin: false,
            placeholder: false,
            visibility: 0,
            cut_away: None,
            cloth: false,
            cloth_mesh: None,
            mesh: false,
            render_flags: 0,
            triangles: None,
        };
        let mut top = 0.0;
        if let Some(rendered) = rendered
            && let Some(bevy_mesh) =
                crate::model::group_mesh(&rendered, &rendered.indices, None, false)
        {
            if let Some((centre, half, radius)) = mesh_edit::bounds(&rendered.positions) {
                root.bounds = (Vec3::from_array(centre), radius);
                root.half_extents = Vec3::from_array(half);
                top = (centre[1] + half[1]) * CM_PER_METRE;
            }
            // The run-time material the original points at the atlas:
            // `_procmatr` when an area's material has an alpha program.
            let alpha = built
                .generated
                .materials
                .iter()
                .any(|m| procedural::material(m.material).passes.alpha);
            root.part_materials = vec![if alpha { "_procmatr" } else { "_procmat1" }.to_string()];
            root.part_flags = rendered
                .triangle_flags
                .as_ref()
                .map(|f| vec![Arc::from(f.as_slice())])
                .unwrap_or_default();
            root.mesh = true;
            debug!(
                "factory weapon {name}: {appended} parts merged into one mesh: {} vertices, {} triangles, {} groups; atlas {:?}",
                rendered.positions.len(),
                rendered.indices.len() / 3,
                rendered.groups.len(),
                crate::atlas::job_size(
                    factory.atlas,
                    self.cache.textures.settings().texture_detail
                ),
            );
            root.parts.push((self.meshes.add(bevy_mesh), material));
        }
        self.store_factory_extents(&factory, &built, weapon, &boxes, &placed, top, key);
        Some(Model { nodes: vec![root] })
    }
}

/// An assembled factory weapon as data, for inspection tools: the merged
/// mesh's texture coordinates and triangles and the painted atlas (base
/// level, RGBA8).
#[derive(Debug, Clone)]
pub struct FactoryPreview {
    pub vertices: usize,
    pub triangles: Vec<[u32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub atlas: (u32, u32),
    pub colour: Vec<u8>,
    pub surface: Vec<u8>,
    pub normal: Option<Vec<u8>>,
}

/// Assembles factory `name` for `weapon` (a fresh recipe when `None`) as
/// the game does and returns its merged mesh and atlas.
pub fn factory_preview(
    game: &crate::GameData,
    name: &str,
    weapon: Option<&Weapon>,
) -> Option<FactoryPreview> {
    let mut cache = crate::model::ModelCache::default();
    let mut meshes = Assets::<Mesh>::default();
    let mut materials = Assets::<ExaMaterial>::default();
    let mut images = Assets::<Image>::default();
    let mut loader = ModelLoader {
        game,
        cache: &mut cache,
        meshes: &mut meshes,
        materials: &mut materials,
        images: &mut images,
        bc_supported: false,
    };
    let model = loader.factory_weapon(name, weapon)?;
    let (mesh, material) = model.nodes.first()?.parts.first()?.clone();
    let mesh = meshes.get(&mesh)?;
    let material = materials.get(&material)?;
    let uvs = match mesh.attribute(Mesh::ATTRIBUTE_UV_0)? {
        bevy::mesh::VertexAttributeValues::Float32x2(v) => v.clone(),
        _ => return None,
    };
    let indices: Vec<u32> = mesh.indices()?.iter().map(|i| i as u32).collect();
    let base = |handle: &Option<Handle<Image>>| -> Option<(u32, u32, Vec<u8>)> {
        let image = images.get(handle.as_ref()?)?;
        let (w, h) = (image.width(), image.height());
        let data = image.data.as_ref()?;
        Some((w, h, data[..(w * h * 4) as usize].to_vec()))
    };
    let (w, h, colour) = base(&material.diffuse_map)?;
    let (_, _, surface) = base(&material.specular_map)?;
    let normal = base(&material.normal_map).map(|(_, _, n)| n);
    Some(FactoryPreview {
        vertices: uvs.len(),
        triangles: indices.as_chunks::<3>().0.to_vec(),
        uvs,
        atlas: (w, h),
        colour,
        surface,
        normal,
    })
}

/// A factory component as the assembler loads it: its first mesh node's
/// mesh, its groups' split flags (from their materials) and the node's
/// stored box (cm).
#[derive(Debug, Clone)]
pub struct FactoryComponent {
    pub mesh: RfcMesh,
    pub split_flags: Vec<u32>,
    pub stored_box: ComponentBox,
}

/// The original's identity frame: the merged mesh is new, so the parts are
/// placed straight into the weapon's space.
const IDENTITY: Frame = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.0; 3]];

/// A placed part's frame in the weapon's space, as the assembler builds the
/// part's matrix: the turn of its angles (X, then Z, then Y; the runtime
/// library's sine and cosine), every row multiplied component-wise by the
/// part's scale (so the scale acts along the weapon's axes, after the
/// turn), and its position (cm, here in metres).
pub fn part_frame(placed: &Placed) -> Frame {
    let [rx, ry, rz] = placed.rotation;
    let turn = exa_physics::engine_math::from_euler(Vec3::new(rx, ry, rz));
    let s = placed.scale;
    let row = |r: bevy::math::Vec3A| [r.x * s[0], r.y * s[1], r.z * s[2]];
    let p = placed.position;
    [
        row(turn.x_axis),
        row(turn.y_axis),
        row(turn.z_axis),
        [
            p[0] / CM_PER_METRE,
            p[1] / CM_PER_METRE,
            p[2] / CM_PER_METRE,
        ],
    ]
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
fn deform_rfc(mesh: &mut RfcMesh, d: &exa_formats::factory::Deform, value: f32) {
    let mut cm: Vec<[f32; 3]> = mesh
        .positions
        .iter()
        .map(|p| p.map(|c| c * CM_PER_METRE))
        .collect();
    deform_positions(&mut cm, d, value);
    for (p, c) in mesh.positions.iter_mut().zip(cm) {
        *p = c.map(|c| c / CM_PER_METRE);
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
    fn a_part_frame_scales_along_the_weapon_after_the_turn() {
        let placed = Placed {
            part: 0,
            variant: 0,
            attach_to: None,
            position: [0.0, 50.0, 0.0],
            rotation: [0.0, std::f32::consts::FRAC_PI_2, 0.0],
            scale: [2.0, 1.0, 1.0],
        };
        let frame = part_frame(&placed);
        let p = exa_formats::morph::place(&frame, [0.0, 0.0, 1.0]);
        // The part's +Z turns onto the weapon's +X and the X scale doubles
        // it; the position is in metres.
        assert!((p[0] - 2.0).abs() < 1e-6, "{p:?}");
        assert!((p[1] - 0.5).abs() < 1e-6 && p[2].abs() < 1e-6, "{p:?}");
    }

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
        let mut missing_shade = std::collections::BTreeSet::new();
        let mut overlapping = std::collections::BTreeSet::new();
        let mut no_mesh = std::collections::BTreeSet::new();
        for name in names {
            let model = loader
                .factory_weapon(&name, None)
                .unwrap_or_else(|| panic!("{name}: no model"));
            // One merged mesh node drawn with one material.
            assert_eq!(model.nodes.len(), 1, "{name}: not merged");
            assert_eq!(model.nodes[0].parts.len(), 1, "{name}: not one mesh");
            let mesh = loader.meshes.get(&model.nodes[0].parts[0].0).unwrap();
            let uvs = mesh
                .attribute(Mesh::ATTRIBUTE_UV_0)
                .and_then(|a| match a {
                    bevy::mesh::VertexAttributeValues::Float32x2(v) => Some(v.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{name}: no texture coordinates"));
            assert!(
                uvs.iter().all(|uv| uv[0].is_finite() && uv[1].is_finite()),
                "{name}: texture coordinates"
            );
            let normals = mesh
                .attribute(Mesh::ATTRIBUTE_NORMAL)
                .and_then(|a| a.as_float3())
                .unwrap();
            assert!(
                normals.iter().all(|n| n.iter().all(|c| c.is_finite())),
                "{name}: normals"
            );
            let (factory, built) = loader.factory_recipe(&name, None).unwrap();
            for (i, part) in factory.parts.iter().enumerate() {
                let sel = built.generated.selection.get(i).copied().unwrap_or(0) as usize;
                if part.flags & 1 != 0 || sel == 0 || sel > part.variants.len() {
                    continue;
                }
                let stem = part.variants[sel - 1].model.to_ascii_lowercase();
                if loader.factory_component(&stem).is_none() {
                    no_mesh.insert(stem.clone());
                }
                if loader.texels(&format!("{stem}d")).is_none() {
                    missing_shade.insert(stem);
                }
            }
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
            // The extents are fractions of the merged mesh's top, and a
            // part's stored box can reach past it: the end can exceed 200 (a
            // real save stores up to 206 for some tools) and a start below
            // the root wraps, as the original stores them.
            loader
                .factory_weapon_stats(&name, None)
                .unwrap_or_else(|| panic!("{name}: no stats"));
            // The chosen parts' regions do not overlap: painting each alone
            // and placing it is then the shared target's draw.
            let size = crate::atlas::job_size(factory.atlas, 3);
            let span = |r: [f32; 4]| {
                let x = [r[0] * size.0 as f32, (r[0] + r[2]) * size.0 as f32];
                let y = [r[1] * size.1 as f32, (r[1] + r[3]) * size.1 as f32];
                (
                    x[0].min(x[1]),
                    x[0].max(x[1]),
                    y[0].min(y[1]),
                    y[0].max(y[1]),
                )
            };
            let boxes = loader.factory_boxes(&factory, &built.generated);
            let placed = factory.place(&built.generated, &boxes);
            for (i, a) in placed.iter().enumerate() {
                for b in &placed[i + 1..] {
                    let (ax0, ax1, ay0, ay1) = span(factory.parts[a.part].atlas);
                    let (bx0, bx1, by0, by1) = span(factory.parts[b.part].atlas);
                    if ax1.min(bx1) - ax0.max(bx0) > 0.5 && ay1.min(by1) - ay0.max(by0) > 0.5 {
                        overlapping.insert(name.clone());
                    }
                }
            }
        }
        // Components drawn without a shade texture keep the cleared atlas.
        println!("components without a shade texture: {missing_shade:?}");
        println!("weapons whose chosen regions overlap: {overlapping:?}");
        // Every chosen component's first node is a mesh.
        assert!(
            no_mesh.is_empty(),
            "components without a first mesh node: {no_mesh:?}"
        );
    }

    /// The striking extents an assembled weapon writes match the ones the
    /// original wrote into a real save's factory weapons (bytes 0x35/0x36,
    /// fractions of the merged mesh's top). Reads the save named by
    /// `EXA_FACTORY_SAVE`; skipped without it or the game.
    #[test]
    fn saved_factory_weapons_keep_their_striking_extents() {
        let (Some(dir), Ok(save)) = (
            crate::locate_install(None),
            std::env::var("EXA_FACTORY_SAVE"),
        ) else {
            return;
        };
        let Ok(bytes) = std::fs::read(save) else {
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
        const HEADER: [u8; 8] = [0, 0x40, 0, 0, 0x68, 0, 0, 0];
        let (mut same, mut same_before, mut checked) = (0, 0, 0);
        for h in 0..bytes.len().saturating_sub(0x68) {
            if bytes[h..h + 8] != HEADER {
                continue;
            }
            // The item's model name sits after the record's 0x100000 tag.
            let from = h.saturating_sub(0x80);
            let Some(j) = bytes[from..h]
                .windows(4)
                .rposition(|w| w == [0, 0, 0x10, 0])
                .map(|j| from + j)
            else {
                continue;
            };
            let name: String = bytes[j + 8..j + 24]
                .iter()
                .take_while(|&&b| b != 0)
                .map(|&b| b as char)
                .collect();
            let Some(factory) = name.strip_prefix('@') else {
                continue;
            };
            let body = &bytes[h + 8..h + 0x68];
            let Some(weapon) = Weapon::parse(body) else {
                continue;
            };
            if weapon.flags & 0x1000_0000 == 0 {
                continue;
            }
            if loader.factory_weapon(factory, Some(&weapon)).is_none() {
                continue;
            }
            let Some(stats) = loader.factory_weapon_stats(factory, Some(&weapon)) else {
                continue;
            };
            checked += 1;
            let got = (
                stats.get(weapon_stats::at::START),
                stats.get(weapon_stats::at::END),
            );
            let want = (body[0x35], body[0x36]);
            // The highest placed box top, the denominator before the merge.
            let (f, built) = loader.factory_recipe(factory, Some(&weapon)).unwrap();
            let boxes = loader.factory_boxes(&f, &built.generated);
            let placed = f.place(&built.generated, &boxes);
            let box_top = placed
                .iter()
                .map(|p| p.position[1] + boxes[p.part].centre[1] + boxes[p.part].half[1])
                .fold(0.0f32, f32::max);
            let (head, _) = ModelLoader::factory_extents(&f, &boxes, &placed);
            let before = head.map(|(b, t)| (extent(b, box_top), extent(t, box_top)));
            if before == Some(want) {
                same_before += 1;
            }
            if got == want {
                same += 1;
            } else {
                println!(
                    "{name} {:#x}: extents {got:?}, saved {want:?}, box top {before:?}",
                    weapon.seeds[0]
                );
            }
        }
        println!("striking extents: {same} of {checked} as saved ({same_before} with the box top)");
        assert!(checked > 0);
    }
}
