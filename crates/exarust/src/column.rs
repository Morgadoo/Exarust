//! `--debug-column x,z`: once the level has loaded, logs every mesh whose
//! bounds cover that floor point, lowest first, with what decides how it
//! shades and shadows: its name path, its height range, its shadow-bias
//! level, whether it casts, into how many cached cubes, and its material's
//! alpha mode and cut. For finding which surface darkens a floor.

use bevy::camera::primitives::Aabb;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use exa_render::cube_shadows::StaticCasters;
use exa_render::lights::ShadowBias;
use exa_render::material::ExaMaterial;
use exa_ui::LoadingStatus;

use crate::Options;

/// Frames to wait after the load, so links and casters are in place.
const SETTLE_FRAMES: u32 = 30;

pub(crate) struct ColumnPlugin;

impl Plugin for ColumnPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            log_column.run_if(|o: Res<Options>| !o.debug_column.is_empty()),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn log_column(
    options: Res<Options>,
    loading: Option<Res<LoadingStatus>>,
    mut waited: Local<Option<u32>>,
    meshes: ColumnMeshes,
    names: Query<&Name>,
    parents: Query<&ChildOf>,
    lights: Query<&StaticCasters>,
    materials: Res<Assets<ExaMaterial>>,
    assets: Res<Assets<Mesh>>,
) {
    if loading.is_some() {
        *waited = Some(0);
        return;
    }
    let Some(frames) = waited.as_mut() else {
        return;
    };
    *frames += 1;
    if *frames != SETTLE_FRAMES {
        return;
    }
    for &[x, z] in &options.debug_column {
        log_point(
            Vec2::new(x, z),
            &meshes,
            &names,
            &parents,
            &lights,
            &materials,
            &assets,
        );
    }
}

type ColumnMeshes<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Mesh3d,
        &'static Aabb,
        &'static GlobalTransform,
        Option<&'static ShadowBias>,
        Has<NotShadowCaster>,
        Option<&'static MeshMaterial3d<ExaMaterial>>,
    ),
>;

fn log_point(
    at: Vec2,
    meshes: &ColumnMeshes,
    names: &Query<&Name>,
    parents: &Query<&ChildOf>,
    lights: &Query<&StaticCasters>,
    materials: &Assets<ExaMaterial>,
    assets: &Assets<Mesh>,
) {
    let mut rows = Vec::new();
    for (e, mesh, aabb, transform, bias, shadowless, material) in meshes {
        let (lo, hi) = world_bounds(aabb, transform);
        if at.x < lo.x || at.x > hi.x || at.y < lo.z || at.y > hi.z {
            continue;
        }
        let path = name_path(e, names, parents);
        let hits = assets
            .get(&mesh.0)
            .map(|m| crossings(m, transform, at))
            .unwrap_or_default();
        let hits = hits
            .iter()
            .map(|(y, up)| format!("{y:.3}{}", if *up { "^" } else { "v" }))
            .collect::<Vec<_>>()
            .join(" ");
        let cubes = lights.iter().filter(|l| l.0.contains(&e)).count();
        let material = material.and_then(|m| materials.get(&m.0)).map_or_else(
            || "no exa material".to_string(),
            |m| {
                format!(
                    "alpha {:?}, cut {}, two-sided {}, opacity {:.2}, flags {:#x}, opacity map {}",
                    m.alpha_mode,
                    m.cuts_shadow(),
                    m.params.flags & exa_render::material::flags::TWO_SIDED != 0,
                    m.params.diffuse.w,
                    m.params.flags,
                    m.opacity_map.is_some(),
                )
            },
        );
        rows.push((
            lo.y,
            format!(
                "{e}: {path} | y {:.3}..{:.3} | surfaces [{hits}] | bias {} | casts {} into {cubes} cubes | {material}",
                lo.y,
                hi.y,
                bias.map_or(0, |b| b.0),
                !shadowless,
            ),
        ));
    }
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    info!("column {},{}: {} meshes", at.x, at.y, rows.len());
    for (_, row) in rows {
        info!("  {row}");
    }
}

/// Where the vertical line at `at` (x, z) crosses the mesh's triangles:
/// each height (m) and whether that triangle faces up.
fn crossings(mesh: &Mesh, transform: &GlobalTransform, at: Vec2) -> Vec<(f32, bool)> {
    let Some(positions) = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .and_then(|a| a.as_float3())
    else {
        return Vec::new();
    };
    let world: Vec<Vec3> = positions
        .iter()
        .map(|p| transform.transform_point(Vec3::from(*p)))
        .collect();
    let indices: Vec<usize> = match mesh.indices() {
        Some(i) => i.iter().collect(),
        None => (0..world.len()).collect(),
    };
    let mut out = Vec::new();
    for t in indices.as_chunks::<3>().0 {
        let (a, b, c) = (world[t[0]], world[t[1]], world[t[2]]);
        // Barycentric coordinates of `at` in the triangle seen from above.
        let d = (b.z - c.z) * (a.x - c.x) + (c.x - b.x) * (a.z - c.z);
        if d.abs() < 1e-12 {
            continue;
        }
        let u = ((b.z - c.z) * (at.x - c.x) + (c.x - b.x) * (at.y - c.z)) / d;
        let v = ((c.z - a.z) * (at.x - c.x) + (a.x - c.x) * (at.y - c.z)) / d;
        let w = 1.0 - u - v;
        if u < 0.0 || v < 0.0 || w < 0.0 {
            continue;
        }
        let y = u * a.y + v * b.y + w * c.y;
        let up = (b - a).cross(c - a).y > 0.0;
        out.push((y, up));
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

/// The world-space box around a mesh's local bounds.
fn world_bounds(aabb: &Aabb, transform: &GlobalTransform) -> (Vec3, Vec3) {
    let (c, h) = (Vec3::from(aabb.center), Vec3::from(aabb.half_extents));
    let mut lo = Vec3::splat(f32::INFINITY);
    let mut hi = Vec3::splat(f32::NEG_INFINITY);
    for i in 0..8 {
        let corner = c + h * Vec3::new(
            if i & 1 == 0 { -1.0 } else { 1.0 },
            if i & 2 == 0 { -1.0 } else { 1.0 },
            if i & 4 == 0 { -1.0 } else { 1.0 },
        );
        let p = transform.transform_point(corner);
        lo = lo.min(p);
        hi = hi.max(p);
    }
    (lo, hi)
}

/// The names from the root down to `e` (unnamed nodes skipped).
fn name_path(e: Entity, names: &Query<&Name>, parents: &Query<&ChildOf>) -> String {
    let mut parts: Vec<String> = names.get(e).map(|n| n.to_string()).into_iter().collect();
    for ancestor in parents.iter_ancestors(e) {
        if let Ok(n) = names.get(ancestor) {
            parts.push(n.to_string());
        }
    }
    parts.reverse();
    if parts.is_empty() {
        "(unnamed)".into()
    } else {
        parts.join(" / ")
    }
}
