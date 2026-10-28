//! What an item icon render needs besides the plan: the camera fitted to the
//! item's bounds, the sun the framing presets build, the scene's baked mesh
//! data and a small CPU rasteriser for headless runs and tests.
//!
//! The running game draws the scene through an off-screen Bevy camera
//! (`exa_game::item_icon_render`); this module owns the numbers both paths
//! share, and paints the same scene when no renderer is present. The camera
//! is the engine's scene fit ([`frame_distance`]) with the icon view's 40°
//! field of view and the plan's framing distance converted from the
//! original's centimetres (`1/100` m, the unit [`exa_assets`] builds models
//! in; the fit is homogeneous in the two lengths). The plan's screen scale,
//! `icon · view_scale / distance · 0.5`, is then the perspective
//! projection's pixel span [guess: the view's scale field is read as
//! `1 / tan(fov / 2)`], so the plan's extents are the camera's own. A small
//! item is bent toward the framing distance and so draws small in its cell;
//! one at least as big as the framing fills it.
//!
//! The framing presets' two numbers are the sun's azimuth and height, which
//! `TSceneEnvironment` turns into a direction with the engine's
//! `(cos(π(1 − h))·cos(πa), sin(π(1 − h)), cos(π(1 − h))·sin(−πa))`.

use bevy::mesh::{Indices, VertexAttributeValues};
use bevy::prelude::*;
use exa_assets::Model;
use exa_render::material::ExaMaterial;

use crate::item_gui::Framing;
use crate::item_renderer::{
    Transform3, extent_scale, frame_distance, projected_box_extents, render_plan, view,
};

/// The icon view's field of view, degrees (its constructor's value).
pub const ICON_FOV_DEG: f32 = 40.0;
/// The original's model unit in metres: its geometry is in centimetres.
pub const FRAMING_CM_PER_M: f32 = 0.01;
/// The ambient share the CPU rasteriser adds to the sun.
pub const ICON_AMBIENT: f32 = 0.2;
/// The rotation the manager's turning variant gives the item, radians.
pub const TURN: [f32; 3] = [-0.1, 0.1, 0.5];

/// A scene's box, in metres about its centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconBounds {
    pub centre: Vec3,
    pub half: Vec3,
}

impl IconBounds {
    /// The box's diagonal radius.
    pub fn radius(&self) -> f32 {
        self.half.length()
    }
}

/// The camera one icon render uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconCamera {
    pub position: Vec3,
    pub target: Vec3,
    pub fov_degrees: f32,
    pub near: f32,
    pub far: f32,
    /// The camera-to-target distance in metres.
    pub distance: f32,
    /// `1 / tan(fov / 2)`: the plan's screen scale factor.
    pub view_scale: f32,
}

impl IconCamera {
    /// The camera the plan frames a box with: on +Z from the box's centre at
    /// the engine's scene-fit distance for the plan's framing (20, 30 or
    /// 70 cm, in metres here) and the icon view's field of view.
    pub fn from_plan(plan: &crate::item_renderer::RenderPlan, bounds: IconBounds) -> Self {
        let fov_degrees = ICON_FOV_DEG;
        let half_angle = fov_degrees.to_radians() * 0.5;
        let view_scale = 1.0 / half_angle.tan();
        let frame = plan.framing.distance * FRAMING_CM_PER_M;
        let distance = frame_distance(bounds.half.to_array(), frame, fov_degrees).max(1.0e-4);
        Self {
            position: bounds.centre + Vec3::new(0.0, 0.0, distance),
            target: bounds.centre,
            fov_degrees,
            near: view(crate::item_renderer::ICON_SIZE).near * FRAMING_CM_PER_M,
            far: 1000.0,
            distance,
            view_scale,
        }
    }

    /// The camera's world rotation (Bevy's `look_at` convention).
    pub fn rotation(&self) -> Quat {
        Transform::from_translation(self.position)
            .looking_at(self.target, Vec3::Y)
            .rotation
    }

    /// The camera's world-to-view matrix (a right-handed look-at, the same
    /// basis Bevy's camera transform gives).
    pub fn view_matrix(&self) -> Mat4 {
        let forward = (self.target - self.position).normalize_or_zero();
        let up = if forward.abs().dot(Vec3::Y) > 0.999 {
            Vec3::Z
        } else {
            Vec3::Y
        };
        let right = forward.cross(up).normalize_or_zero();
        let true_up = right.cross(forward);
        Mat4::from_cols(
            Vec4::new(right.x, true_up.x, -forward.x, 0.0),
            Vec4::new(right.y, true_up.y, -forward.y, 0.0),
            Vec4::new(right.z, true_up.z, -forward.z, 0.0),
            Vec4::new(
                -right.dot(self.position),
                -true_up.dot(self.position),
                forward.dot(self.position),
                1.0,
            ),
        )
    }

    /// The view matrix as the renderer's packed 3 × 4 rows.
    pub fn view_rows(&self) -> Transform3 {
        let m = self.view_matrix();
        let r = |i: usize| {
            let row = m.row(i);
            [row.x, row.y, row.z, row.w]
        };
        [r(0), r(1), r(2)]
    }

    /// The box's projected extents in the original's centimetres: the
    /// extents helper widens each span by 0.0001 of the original's unit, so
    /// the box and the view's translation are scaled from metres first.
    pub fn box_extents_cm(&self, bounds: IconBounds) -> (f32, f32) {
        let cm = 1.0 / FRAMING_CM_PER_M;
        let mut rows = self.view_rows();
        for row in &mut rows {
            row[3] *= cm;
        }
        projected_box_extents(
            (bounds.centre * cm).to_array(),
            (bounds.half * cm).to_array(),
            &rows,
        )
    }

    /// The camera-to-target distance squared, in centimetres.
    pub fn distance_squared_cm(&self) -> f32 {
        let d = self.distance / FRAMING_CM_PER_M;
        d * d
    }

    /// The pixel extents the plan gives a box at this camera.
    pub fn pixel_extents(&self, bounds: IconBounds, icon_pixels: i32) -> (f32, f32) {
        let extents = self.box_extents_cm(bounds);
        let scale = extent_scale(icon_pixels, self.view_scale, self.distance_squared_cm());
        (extents.0 * scale, extents.1 * scale)
    }
}

/// The render plan for a scene at this camera and plan variant: the framing
/// camera first, then the plan with the box's projected extents and the
/// camera's own distance and scale. `alpha` is the alpha global (the screen
/// pass picks it over the downsample and halo).
#[allow(clippy::too_many_arguments)]
pub fn scene_plan(
    variant: u32,
    close: bool,
    model_flags: u32,
    bounds: IconBounds,
    icon_pixels: i32,
    alpha: bool,
) -> (IconCamera, crate::item_renderer::RenderPlan) {
    let first = crate::item_renderer::render_plan(
        variant,
        close,
        (0.0, 0.0),
        model_flags,
        icon_pixels,
        1.0,
        1.0,
        alpha,
    );
    let camera = IconCamera::from_plan(&first, bounds);
    let plan = render_plan(
        variant,
        close,
        camera.box_extents_cm(bounds),
        model_flags,
        icon_pixels,
        camera.view_scale,
        camera.distance_squared_cm(),
        alpha,
    );
    (camera, plan)
}

/// The sun direction the framing preset's two numbers build, from the
/// scene environment's sector:
/// `x = cos(π(1 − h))·cos(−πa)`, `y = sin(π(1 − h))`,
/// `z = cos(π(1 − h))·sin(−πa)`. The engine's fast sine and cosine are
/// approximated here by the standard library's [guess: the difference is
/// below the framing presets' noise].
pub fn sun_direction(framing: Framing) -> Vec3 {
    let azimuth = framing.sun.0;
    let height = framing.sun.1;
    let vertical = (1.0 - height) * std::f32::consts::PI;
    let across = -azimuth * std::f32::consts::PI;
    let flat = vertical.cos();
    Vec3::new(flat * across.cos(), vertical.sin(), flat * across.sin())
}

/// The rotation the turned variant gives the item.
pub fn turn_rotation() -> Quat {
    Quat::from_euler(EulerRot::XYZ, TURN[0], TURN[1], TURN[2])
}

/// One mesh of a scene, for the off-screen camera: its handle, material and
/// absolute transform in the item's frame (before the turn and centring).
#[derive(Debug, Clone)]
pub struct IconPart {
    pub mesh: Handle<Mesh>,
    pub material: Handle<ExaMaterial>,
    pub transform: Mat4,
}

/// A mesh baked for the CPU rasteriser: centred vertex positions and
/// normals, indices and the material's diffuse and emissive colours.
#[derive(Debug, Clone)]
pub struct IconSurface {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub diffuse: [f32; 4],
    pub emissive: [f32; 3],
}

/// An item ready to draw: the camera-spawnable parts and the baked meshes.
#[derive(Debug, Clone)]
pub struct IconScene {
    pub parts: Vec<IconPart>,
    pub surfaces: Vec<IconSurface>,
    pub bounds: IconBounds,
    /// The box's centre before centring: the off-screen root moves by its
    /// negative.
    pub centre: Vec3,
    pub turn: Quat,
}

impl IconScene {
    /// Bakes a model's drawable meshes for an icon. `hide_star` drops the
    /// parts whose material name begins with `*` (the manager's model-flag
    /// rule); `turned` applies the variant's rotation. `None` when the model
    /// has no mesh with positions and triangles.
    pub fn from_model(
        model: &Model,
        meshes: &Assets<Mesh>,
        materials: &Assets<ExaMaterial>,
        hide_star: bool,
        turned: bool,
    ) -> Option<Self> {
        let turn = if turned {
            turn_rotation()
        } else {
            Quat::IDENTITY
        };
        let root = Mat4::from_quat(turn);
        let mut cache = vec![None; model.nodes.len()];
        let mut visiting = vec![false; model.nodes.len()];
        let mut absolute = vec![Mat4::IDENTITY; model.nodes.len()];
        for (index, slot) in absolute.iter_mut().enumerate() {
            *slot = absolute_matrix(model, index, &mut cache, &mut visiting);
        }
        let mut parts = Vec::new();
        let mut surfaces = Vec::new();
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for (index, node) in model.nodes.iter().enumerate() {
            if node.placeholder {
                continue;
            }
            let world = root * absolute[index];
            for (part, (handle, material)) in node.parts.iter().enumerate() {
                if hide_star
                    && node
                        .part_materials
                        .get(part)
                        .is_some_and(|name| name.starts_with('*'))
                {
                    continue;
                }
                let Some(mesh) = meshes.get(handle) else {
                    continue;
                };
                let Some(positions) = mesh
                    .attribute(Mesh::ATTRIBUTE_POSITION)
                    .and_then(VertexAttributeValues::as_float3)
                else {
                    continue;
                };
                let indices = match mesh.indices() {
                    Some(Indices::U16(v)) => v.iter().map(|&i| u32::from(i)).collect(),
                    Some(Indices::U32(v)) => v.clone(),
                    None => continue,
                };
                if positions.is_empty() || indices.is_empty() {
                    continue;
                }
                let normals = mesh
                    .attribute(Mesh::ATTRIBUTE_NORMAL)
                    .and_then(VertexAttributeValues::as_float3)
                    .filter(|n| n.len() == positions.len())
                    .map(<[[f32; 3]]>::to_vec);
                let (diffuse, emissive) =
                    materials
                        .get(material)
                        .map_or(([0.7, 0.7, 0.7, 1.0], [0.0; 3]), |m| {
                            let e = m.params.emissive;
                            (m.params.diffuse.to_array(), [e.x, e.y, e.z])
                        });
                let baked: Vec<[f32; 3]> = positions
                    .iter()
                    .map(|p| world.transform_point3(Vec3::from(*p)).to_array())
                    .collect();
                let baked_normals: Vec<[f32; 3]> = normals
                    .as_ref()
                    .map(|n| {
                        n.iter()
                            .map(|v| {
                                world
                                    .transform_vector3(Vec3::from(*v))
                                    .normalize_or_zero()
                                    .to_array()
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                for p in &baked {
                    let p = Vec3::from(*p);
                    min = min.min(p);
                    max = max.max(p);
                }
                parts.push(IconPart {
                    mesh: handle.clone(),
                    material: material.clone(),
                    transform: absolute[index],
                });
                surfaces.push(IconSurface {
                    positions: baked,
                    normals: baked_normals,
                    indices,
                    diffuse,
                    emissive,
                });
            }
        }
        if surfaces.is_empty() {
            return None;
        }
        let centre = (min + max) * 0.5;
        let bounds = IconBounds {
            centre: Vec3::ZERO,
            half: (max - min) * 0.5,
        };
        for surface in &mut surfaces {
            for p in &mut surface.positions {
                *p = (Vec3::from(*p) - centre).to_array();
            }
        }
        Some(Self {
            parts,
            surfaces,
            bounds,
            centre,
            turn,
        })
    }

    /// A two-triangle white square in the XY plane, one metre across: a
    /// synthetic scene for a headless App or a test that only needs a
    /// pending icon to become a cell.
    pub fn synthetic_quad() -> Self {
        Self {
            parts: Vec::new(),
            surfaces: vec![IconSurface {
                positions: vec![
                    [-0.5, -0.5, 0.0],
                    [0.5, -0.5, 0.0],
                    [0.5, 0.5, 0.0],
                    [-0.5, 0.5, 0.0],
                ],
                normals: vec![[0.0, 0.0, 1.0]; 4],
                indices: vec![0, 1, 2, 0, 2, 3],
                diffuse: [0.8, 0.8, 0.8, 1.0],
                emissive: [0.0; 3],
            }],
            bounds: IconBounds {
                centre: Vec3::ZERO,
                half: Vec3::new(0.5, 0.5, 0.0),
            },
            centre: Vec3::ZERO,
            turn: Quat::IDENTITY,
        }
    }

    /// Paints the scene from `camera` into an RGBA image of `size` pixels,
    /// rows bottom-up (the atlas's order). The sun is the direction toward
    /// the light. This is the headless path; the running game's camera uses
    /// the same scene and numbers.
    pub fn paint(&self, camera: &IconCamera, sun: Vec3, size: u32) -> IconPixels {
        let size = size.max(1);
        let width = size as usize;
        let height = size as usize;
        let mut pixels = vec![0u8; width * height * 4];
        let mut depth = vec![1.0f32; width * height];
        let view = camera.view_matrix();
        let projection = bevy::math::proj::perspective(
            camera.fov_degrees.to_radians(),
            1.0,
            camera.near,
            camera.far,
        );
        let view_projection = projection * view;
        let light = sun.normalize_or_zero();
        for surface in &self.surfaces {
            for triangle in surface.indices.as_chunks::<3>().0 {
                let [ia, ib, ic] = [
                    triangle[0] as usize,
                    triangle[1] as usize,
                    triangle[2] as usize,
                ];
                if ia >= surface.positions.len()
                    || ib >= surface.positions.len()
                    || ic >= surface.positions.len()
                {
                    continue;
                }
                let a = Vec3::from(surface.positions[ia]);
                let b = Vec3::from(surface.positions[ib]);
                let c = Vec3::from(surface.positions[ic]);
                let mut normal = (b - a).cross(c - a).normalize_or_zero();
                if normal == Vec3::ZERO {
                    continue;
                }
                // Face the eye: models are drawn two-sided.
                if normal.dot(camera.position - a) < 0.0 {
                    normal = -normal;
                }
                let ndotl = normal.dot(light).max(0.0);
                let lit = Vec3::new(surface.diffuse[0], surface.diffuse[1], surface.diffuse[2]);
                let emissive = Vec3::from_array(surface.emissive);
                let colour = lit * (ICON_AMBIENT + (1.0 - ICON_AMBIENT) * ndotl) + emissive;
                let alpha = surface.diffuse[3].clamp(0.0, 1.0);
                let mut screen = [[0.0f32; 3]; 3];
                let mut behind = false;
                for (slot, point) in [a, b, c].into_iter().enumerate() {
                    let p = view_projection * point.extend(1.0);
                    if p.w <= 1.0e-6 {
                        behind = true;
                        break;
                    }
                    let ndc = p.truncate() / p.w;
                    screen[slot] = [
                        (ndc.x * 0.5 + 0.5) * width as f32,
                        (0.5 - ndc.y * 0.5) * height as f32,
                        ndc.z,
                    ];
                }
                if behind {
                    continue;
                }
                let [pa, pb, pc] = screen;
                let area = (pb[0] - pa[0]) * (pc[1] - pa[1]) - (pb[1] - pa[1]) * (pc[0] - pa[0]);
                if area.abs() < 1.0e-8 {
                    continue;
                }
                let min_x = pa[0].min(pb[0]).min(pc[0]).floor().max(0.0) as usize;
                let max_x = (pa[0].max(pb[0]).max(pc[0]).ceil() as i64).min(width as i64 - 1);
                let min_y = pa[1].min(pb[1]).min(pc[1]).floor().max(0.0) as usize;
                let max_y = (pa[1].max(pb[1]).max(pc[1]).ceil() as i64).min(height as i64 - 1);
                if max_x < min_x as i64 || max_y < min_y as i64 {
                    continue;
                }
                for y in min_y..=max_y as usize {
                    for x in min_x..=max_x as usize {
                        let px = x as f32 + 0.5;
                        let py = y as f32 + 0.5;
                        let w0 = ((pb[0] - px) * (pc[1] - py) - (pb[1] - py) * (pc[0] - px)) / area;
                        let w1 = ((pc[0] - px) * (pa[1] - py) - (pc[1] - py) * (pa[0] - px)) / area;
                        let w2 = 1.0 - w0 - w1;
                        if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                            continue;
                        }
                        let z = w0 * pa[2] + w1 * pb[2] + w2 * pc[2];
                        let at = y * width + x;
                        if z >= depth[at] {
                            continue;
                        }
                        depth[at] = z;
                        let row = height - 1 - y;
                        let pixel = (row * width + x) * 4;
                        let src = [colour.x, colour.y, colour.z, alpha];
                        let dst = [
                            f32::from(pixels[pixel]) / 255.0,
                            f32::from(pixels[pixel + 1]) / 255.0,
                            f32::from(pixels[pixel + 2]) / 255.0,
                            f32::from(pixels[pixel + 3]) / 255.0,
                        ];
                        let out_a = src[3] + dst[3] * (1.0 - src[3]);
                        for i in 0..3 {
                            let mixed = if out_a > 0.0 {
                                (src[i] * src[3] + dst[i] * dst[3] * (1.0 - src[3])) / out_a
                            } else {
                                src[i]
                            };
                            pixels[pixel + i] = srgb_byte(mixed);
                        }
                        pixels[pixel + 3] = (out_a * 255.0).round() as u8;
                    }
                }
            }
        }
        IconPixels {
            width: size,
            height: size,
            pixels,
        }
    }
}

/// A node's absolute transform in the model. A parent index past the node's
/// own, or a parent cycle in malformed data, is read as the node's own
/// transform, so the walk cannot loop or recurse forever.
fn absolute_matrix(
    model: &Model,
    index: usize,
    cache: &mut [Option<Mat4>],
    visiting: &mut [bool],
) -> Mat4 {
    if let Some(done) = cache[index] {
        return done;
    }
    let local = model.nodes[index].transform.to_matrix();
    let done = match model.nodes[index].parent {
        Some(parent) if parent < model.nodes.len() && !visiting[parent] => {
            visiting[index] = true;
            let parent_matrix = absolute_matrix(model, parent, cache, visiting);
            visiting[index] = false;
            parent_matrix * local
        }
        _ => local,
    };
    cache[index] = Some(done);
    done
}

/// Encodes a linear channel as sRGB.
fn srgb_byte(value: f32) -> u8 {
    let c = value.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round() as u8
}

/// An RGBA image with its rows bottom-up, as the icon atlas stores them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconPixels {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl IconPixels {
    /// Whether any pixel has a non-zero alpha.
    pub fn any_alpha(&self) -> bool {
        self.pixels.as_chunks::<4>().0.iter().any(|p| p[3] != 0)
    }

    /// The alpha at `(x, y)`, counted from the bottom row.
    pub fn alpha(&self, x: u32, y: u32) -> u8 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        let at = (y as usize * self.width as usize + x as usize) * 4;
        self.pixels.get(at + 3).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icon_cache::variant as v;

    fn bounds(half: Vec3) -> IconBounds {
        IconBounds {
            centre: Vec3::ZERO,
            half,
        }
    }

    #[test]
    fn the_camera_is_the_engines_scene_fit() {
        let plan = crate::item_renderer::render_plan(
            v::APPAREL,
            false,
            (1.0, 1.0),
            0,
            160,
            1.0,
            1.0,
            true,
        );
        // 40°: 1/tan(20°).
        let view_scale = 1.0 / (20.0f32.to_radians()).tan();
        assert!((view_scale - 2.747_477).abs() < 1.0e-5);
        // A 10 cm box at the 30 cm apparel framing: r = 1/6, the half bends
        // to 0.3 · ((r · 0.11 + 1) − 1/(8r + 1)) = 0.176 93, over tan(20°)
        // plus the depth half 0.05.
        let camera = IconCamera::from_plan(&plan, bounds(Vec3::new(0.05, 0.05, 0.05)));
        assert_eq!(camera.fov_degrees, 40.0);
        assert!(
            (camera.distance - 0.536_107).abs() < 1.0e-5,
            "{}",
            camera.distance
        );
        assert!((camera.position - Vec3::new(0.0, 0.0, camera.distance)).length() < 1.0e-6);
        assert!((camera.view_scale - view_scale).abs() < 1.0e-5);
        assert!((camera.near - 0.1).abs() < 1.0e-6);
        // A small item draws small: 41 of 160 pixels.
        let (w, _) = camera.pixel_extents(bounds(Vec3::new(0.05, 0.05, 0.05)), 160);
        assert!((w - 40.999).abs() < 0.01, "{w}");
        // A 1 m box is above the framing, so its half is used as it is:
        // 0.5 / tan(20°) + 0.5.
        let big = bounds(Vec3::new(0.5, 0.5, 0.5));
        let camera = IconCamera::from_plan(&plan, big);
        assert!((camera.distance - 1.873_739).abs() < 1.0e-5);
        let (w, h) = camera.pixel_extents(big, 160);
        assert!(
            (w - 117.305).abs() < 0.01 && (h - 117.305).abs() < 0.01,
            "{w} {h}"
        );
        // The close (20 cm) and default (70 cm) framings.
        let close = crate::item_renderer::render_plan(0, true, (1.0, 1.0), 0, 160, 1.0, 1.0, true);
        let d = IconCamera::from_plan(&close, bounds(Vec3::splat(0.02))).distance;
        assert!((d - 0.270_265).abs() < 1.0e-5, "{d}");
        let far = crate::item_renderer::render_plan(0, false, (1.0, 1.0), 0, 160, 1.0, 1.0, true);
        let d = IconCamera::from_plan(&far, bounds(Vec3::splat(0.02))).distance;
        assert!((d - 0.383_855).abs() < 1.0e-5, "{d}");
    }

    #[test]
    fn the_scene_plan_repeats_the_extents_in_pixels() {
        let bounds = bounds(Vec3::new(0.5, 0.5, 0.5));
        let (_, plan) = scene_plan(v::APPAREL, false, 0, bounds, 160, true);
        assert_eq!(plan.framing.distance, 30.0);
        assert!((plan.screen_extents.0 - 117.305).abs() < 0.01);
        assert!((plan.screen_extents.1 - 117.305).abs() < 0.01);
        assert!(plan.screen_pass && !plan.downsample && !plan.halo);
        // The same at the icon's own size.
        let (_, small) = scene_plan(v::APPAREL, false, 0, bounds, 80, false);
        assert!((small.screen_extents.0 - 58.652).abs() < 0.01);
        assert!(!small.screen_pass && small.downsample && small.halo);
    }

    #[test]
    fn the_sun_preset_builds_the_scene_direction() {
        let weapon = crate::item_gui::framing(v::WEAPON | v::APPAREL, false);
        let d = sun_direction(Framing {
            sun: (-0.21, 0.7516),
            ..weapon
        });
        // sin(π · 0.2484) and cos(π · 0.2484) · cos/sin(π · 0.21).
        assert!((d.y - 0.703_66).abs() < 0.0002);
        assert!((d.x - 0.561_6).abs() < 0.002);
        assert!((d.z - 0.435_3).abs() < 0.002);
        let other = sun_direction(crate::item_gui::framing(v::WEAPON, false));
        assert!((other.y - 0.889_2).abs() < 0.002);
        assert!(other.length() > 0.99 && other.length() < 1.01);
    }

    #[test]
    fn the_turn_is_the_variant_rotation() {
        let q = turn_rotation();
        let turned = q * Vec3::Z;
        // The turn leans the item's +Z toward +X and +Y; the half radian
        // turn about Z is the largest term.
        assert!((turned.x - 0.099_83).abs() < 1.0e-4, "{turned:?}");
        assert!((turned.y - 0.099_33).abs() < 1.0e-4, "{turned:?}");
        assert!((turned.z - 0.990_03).abs() < 1.0e-4, "{turned:?}");
    }

    /// A shipped weapon's ground model paints a non-empty icon. Skipped
    /// without an installation.
    #[test]
    fn a_shipped_weapons_icon_renders_non_empty() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let Ok(game) = exa_assets::GameData::open(&dir) else {
            return;
        };
        let Some(bytes) = game
            .pack(exa_assets::PackKind::Resource)
            .and_then(|p| p.get("objdb.rdb"))
        else {
            return;
        };
        let Ok(db) = exa_formats::rdb::Database::parse(bytes) else {
            return;
        };
        let items = exa_formats::objdb::items(&db);
        let item = items
            .iter()
            .find(|i| i.model.starts_with('@') && i.weapon.is_some())
            .expect("the shipped object database holds a factory weapon");
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<ExaMaterial>::default();
        let mut images = Assets::<Image>::default();
        let mut cache = exa_assets::ModelCache::default();
        let mut loader = exa_assets::ModelLoader {
            game: &game,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: false,
        };
        let model = loader
            .factory_weapon(item.model.trim_start_matches('@'), item.weapon.as_ref())
            .expect("a shipped factory weapon assembles");
        let scene = IconScene::from_model(&model, &meshes, &materials, false, false)
            .expect("a shipped weapon has drawable meshes");
        let variant = if item.model_flags & 1 != 0 {
            crate::icon_cache::variant::LIT
        } else {
            0
        };
        let (camera, plan) = scene_plan(variant, false, item.model_flags, scene.bounds, 160, true);
        let icon = scene.paint(
            &camera,
            sun_direction(crate::item_gui::framing(variant, false)),
            160,
        );
        assert!(plan.screen_pass);
        assert!(icon.any_alpha(), "{} has an empty icon", item.name);
        assert!(plan.screen_extents.0 > 0.0 && plan.screen_extents.1 > 0.0);
    }

    #[test]
    fn the_synthetic_scene_paints_opaque_pixels() {
        let scene = IconScene::synthetic_quad();
        let (camera, _) = scene_plan(0, false, 0, scene.bounds, 160, true);
        let icon = scene.paint(
            &camera,
            sun_direction(crate::item_gui::framing(0, false)),
            160,
        );
        assert_eq!((icon.width, icon.height), (160, 160));
        assert!(icon.any_alpha());
        let centre = icon.alpha(80, 80);
        assert_eq!(centre, 255);
        assert_eq!(icon.alpha(0, 0), 0);
        // A 1 m square is above the 70 cm framing, so it fills about 123 of
        // the 160 pixels and leaves the border clear.
        let mut painted = 0;
        for y in 0..160 {
            for x in 0..160 {
                if icon.alpha(x, y) != 0 {
                    painted += 1;
                }
            }
        }
        assert!(painted > 10_000, "{painted} pixels");
    }
}
