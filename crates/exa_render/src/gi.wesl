// One step of the original's voxel global illumination, a light-propagation
// volume: each open voxel gathers its eight diagonal neighbours' light
// (weighted by how open they are), keeps most of it, gains a faint base
// ambient and the direct light of its sector's lights; point lights are
// shadowed by their cube maps and the sun by its orthographic map. Voxel
// lights (glowing items) then add their light straight into the voxels
// around them. Light builds up over many steps. Storage is 8-bit sRGB like
// the original's.
//
// The original rasterises one quad per slice with a layered framebuffer and
// reads its 3D light texture at the slice boundary (so linear filtering
// averages the two neighbouring slices); this compute pass writes the same
// voxel of each slice and samples at the same boundary coordinates.
// Rules: docs/RENDERING.md (Voxel GI).

struct GiLight {
    // A point light's position in texture units (x, z depth, y height) and
    // the inverse of its radius in those units. Unused for the sun.
    position_radius: vec4<f32>,
    // Per-step injection colour: linear colour × intensity × 0.15 (×0.9 for
    // the sun), before the fragment's 0.01.
    colour: vec4<f32>,
    // x, y: the light's vertical shaping (y, z); z: 1/(2^z − 1) (unshaped
    // lights carry (1, 1, 1)); w: 0 for a point light, 1 for the sun.
    shaping: vec4<f32>,
    // x: the light's cube slot + 1 (0 without a shadow map).
    extra: vec4<u32>,
}

struct GiVoxelLight {
    // The light in texture units (x, z depth, y height).
    position: vec4<f32>,
    // Per-step colour (linear colour × √intensity × 3 × 0.1).
    colour: vec4<f32>,
    // The voxels it reaches: lo up to but not including hi.
    lo: vec4<u32>,
    hi: vec4<u32>,
}

// A group of sectors drawn together: the slice it draws from, and the run of
// lights its sectors' cells record.
struct GiGroup {
    floor: u32,
    first: u32,
    count: u32,
    _pad: u32,
}

struct GiParams {
    // Lowest corner of the block (m).
    origin: vec4<f32>,
    // Size of the block (m).
    extent: vec4<f32>,
    // The base ambient every open voxel gains per step, per channel.
    base: vec4<f32>,
    // The sun's fitted shadow matrix, world (m) to Bevy's clip space.
    sun_matrix: mat4x4<f32>,
    // x: 1 when the sun shadow map may be sampled, y: 1 under an open sky.
    sun_flags: vec4<u32>,
    groups: array<GiGroup, 64>,
    // Per sector (X fastest), its group or 0xffffffff.
    sector_groups: array<vec4<u32>, 16>,
    lights: array<GiLight, 768>,
    // x = the number of voxel lights.
    voxel_count: vec4<u32>,
    voxel_lights: array<GiVoxelLight, 64>,
}

const SECTORS: u32 = 8u;
const SECTOR_VOXELS: u32 = 30u;
const MARGIN: u32 = 8u;
const NO_GROUP: u32 = 0xffffffffu;
// The original samples its shadow cube maps with world Z flipped.
const FLIP_Z: vec3<f32> = vec3(1.0, 1.0, -1.0);

@group(0) @binding(0) var light_prev: texture_3d<f32>;
@group(0) @binding(1) var occupancy: texture_3d<f32>;
@group(0) @binding(2) var linear_sampler: sampler;
@group(0) @binding(3) var light_next: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(4) var<uniform> params: GiParams;
// The cached point-light cubes (the original's `Shadow<i>` cube maps).
@group(0) @binding(5) var shadow_cubes: texture_depth_cube_array;
@group(0) @binding(6) var shadow_cube_sampler: sampler_comparison;
// The sun's fitted orthographic depth map (the original's `Shadow0`).
@group(0) @binding(7) var sun_shadow: texture_depth_2d_array;
@group(0) @binding(8) var sun_shadow_sampler: sampler_comparison;

fn to_srgb(c: vec3<f32>) -> vec3<f32> {
    let l = clamp(c, vec3(0.0), vec3(1.0));
    return select(1.055 * pow(l, vec3(1.0 / 2.4)) - 0.055, l * 12.92, l <= vec3(0.0031308));
}

fn openness(occ: f32) -> f32 {
    return max(1.0 - 13.0 * occ, 0.0);
}

// The slice the original's layered quad writes and the texture coordinates
// it reads: x and y at the texel centres, z on the slice boundary.
fn volume_uv(id: vec3<u32>, dims: vec3<u32>) -> vec3<f32> {
    return vec3(
        (f32(id.x) + 0.5) / f32(dims.x),
        (f32(id.y) + 0.5) / f32(dims.y),
        f32(id.z) / f32(dims.z),
    );
}

// The original's world-scale volume coordinates: x and y as texture units,
// z the height divided by the block's width (not its slice count), so one
// vertical voxel is one 1/width step.
fn volume_height(id: vec3<u32>, dims: vec3<u32>) -> vec3<f32> {
    return vec3(
        (f32(id.x) + 0.5) / f32(dims.x),
        (f32(id.y) + 0.5) / f32(dims.y),
        f32(id.z) / f32(dims.x),
    );
}

// What the voxel lights add to voxel `id` this step (the original's
// `gi_emission` pass, one light at a time). Each reaches the box the CPU
// worked out for it and falls off as max(1 − 5000·d², 0)⁴, with d in
// world-scale texture units, nothing beyond 3.62 voxels. Geometry at the
// voxel or halfway to the light (openness 1 − occupancy, unlike the
// propagation's) leaves only the voxels within about 1.2 of the light lit.
fn voxel_lights(id: vec3<u32>, dims: vec3<u32>) -> vec3<f32> {
    var rgb = vec3(0.0);
    let uv = volume_uv(id, dims);
    let at = volume_height(id, dims);
    let here = 1.0 - textureSampleLevel(occupancy, linear_sampler, uv, 0.0).r;
    for (var i = 0u; i < params.voxel_count.x; i++) {
        let lt = params.voxel_lights[i];
        if any(id < lt.lo.xyz) || any(id >= lt.hi.xyz) {
            continue;
        }
        var o = lt.position.xyz;
        o.z = o.z * f32(dims.x) / f32(dims.z);
        let mid = mix(o, uv, 0.5);
        let around = 1.0 - textureSampleLevel(occupancy, linear_sampler, mid, 0.0).r;
        let occ = min((1.0 - here * around) * 20.0, 1.0);
        let q = at - lt.position.xyz;
        let atn = pow(max(1.0 - dot(q, q) * 5000.0, 0.0), 4.0);
        let near = max(2.0 - 0.75 / (atn * atn), 0.0);
        let blocked = 1.0 - occ * (1.0 - near);
        rgb += lt.colour.rgb * atn * 0.25 * blocked;
    }
    return rgb;
}

// The voxels the manager does not draw this frame: the original clears the
// scratch target to (0, 0, 0, 0) and then runs the voxel lights' additive
// pass over it, so they take the lights' colour and no openness.
fn write_dark(id: vec3<u32>, dims: vec3<u32>) {
    textureStore(light_next, vec3<i32>(id), vec4(to_srgb(voxel_lights(id, dims)), 0.0));
}

// The sun's visibility at `at`: the fitted shadow map's comparison, with the
// original's two-sample difference when the level has no sky, and the
// voxel's openness. Without a shadow map the sun is unshadowed.
fn sun_light(at: vec3<f32>, occ: f32) -> f32 {
    if params.sun_flags.x == 0u {
        return occ;
    }
    let clip = params.sun_matrix * vec4(at, 1.0);
    if clip.w <= 0.0 {
        return occ;
    }
    let ndc = clip.xyz / clip.w;
    if any(ndc.xy < vec2(-1.0)) || any(ndc.xy > vec2(1.0)) || ndc.z < 0.0 || ndc.z > 1.0 {
        return occ;
    }
    let uv = vec2(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    let first = textureSampleCompareLevel(sun_shadow, sun_shadow_sampler, uv, 0, ndc.z);
    if params.sun_flags.y != 0u {
        return first * occ;
    }
    // The original's second sample is 0.01 deeper; Bevy's map is reverse-Z,
    // so deeper is smaller. [guess: the depth scale of 0.01 is the
    // original's map's, kept as is]
    let second = textureSampleCompareLevel(sun_shadow, sun_shadow_sampler, uv, 0, ndc.z - 0.01);
    return max(first - second, 0.0) * occ;
}

@compute @workgroup_size(4, 4, 4)
fn propagate(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(light_next);
    if any(id >= dims) {
        return;
    }
    // The 8-voxel margins, and the top slice drawn by no group, are cleared
    // every step; slices outside a sector group's span and unmarked sectors
    // are too.
    if id.x < MARGIN || id.y < MARGIN || id.x + MARGIN >= dims.x || id.y + MARGIN >= dims.y {
        write_dark(id, dims);
        return;
    }
    let sector = vec2<u32>((id.x - MARGIN) / SECTOR_VOXELS, (id.y - MARGIN) / SECTOR_VOXELS);
    let index = sector.x + sector.y * SECTORS;
    let group = params.sector_groups[index / 4u][index % 4u];
    if group == NO_GROUP {
        write_dark(id, dims);
        return;
    }
    let plan = params.groups[group];
    if id.z < plan.floor || id.z + 1u >= dims.z {
        write_dark(id, dims);
        return;
    }
    let uvw = volume_uv(id, dims);
    let at = volume_height(id, dims);
    // The light is sampled through an sRGB view: linear, decoded before
    // filtering.
    let old = textureSampleLevel(light_prev, linear_sampler, uvw, 0.0);
    let occ0 = textureSampleLevel(occupancy, linear_sampler, uvw, 0.0).r;
    let stored_alpha = clamp(1.0 - 5.0 * occ0, 0.0, 1.0);
    var open = openness(occ0);

    // The program variant the original builds for a block depends on whether
    // its first light is the sun: outdoors a neighbour's light counts by its
    // openness, indoors by its openness squared.
    var direct = false;
    if plan.count > 0u {
        direct = params.lights[plan.first].shaping.w == 1.0;
    }
    var sum = vec3(0.0);
    var s = 0.0;
    let step = vec3(1.33) / vec3(f32(dims.x), f32(dims.y), f32(dims.z));
    // The original's corner order: x fastest, then y, then z.
    for (var i = 0u; i < 8u; i++) {
        let corner = vec3<f32>(f32(i & 1u), f32((i >> 1u) & 1u), f32((i >> 2u) & 1u)) * 2.0 - 1.0;
        let n = uvw + corner * step;
        let a = openness(textureSampleLevel(occupancy, linear_sampler, n, 0.0).r);
        s += a;
        var p = textureSampleLevel(light_prev, linear_sampler, n, 0.0).rgb;
        p *= select(a * a, a, direct);
        sum += mix(old.rgb, p, a);
    }
    // The lowest slice fades out.
    let fade = exp2(-16.0 * uvw.z);
    open *= 1.0 - fade;
    let k = min(8.0 * s, 1.0) * (0.3 * open + 0.699);
    var rgb = sum * k * 0.125 + params.base.rgb * open;

    // Direct light, from the lights of this sector's group only. Texture
    // axes are world X, Z, Y.
    for (var i = 0u; i < plan.count; i++) {
        let lt = params.lights[plan.first + i];
        if lt.shaping.w == 1.0 {
            // The sun: its light is the fitted world position through the
            // shadow matrix (the original's `SdwMat` path).
            // `at` is in texture axes (x, z depth, height over width);
            // `extent` and `origin` in world axes (X, height, Z).
            let world = params.origin.xyz
                + vec3(at.x * params.extent.x, at.z * params.extent.x, at.y * params.extent.z);
            rgb += lt.colour.rgb * sun_light(world, open) * 0.01;
            continue;
        }
        let to = at - lt.position_radius.xyz;
        let d = length(to);
        var shadow = 1.0;
        if lt.extra.x != 0u {
            let dir = normalize(vec3(to.x, to.z, to.y)) * FLIP_Z;
            shadow = textureSampleCompareLevel(
                shadow_cubes,
                shadow_cube_sampler,
                dir,
                i32(lt.extra.x - 1u),
                d * lt.position_radius.w + 0.01,
            );
        }
        var atn = max(1.0 - d * lt.position_radius.w, 0.0);
        atn = atn * atn;
        atn = atn * atn * open * shadow;
        let ly = -to.z / d;
        let shaped = max((exp2((ly + (1.0 - ly) * lt.shaping.x) * lt.shaping.y) - 1.0) * lt.shaping.z, 0.0);
        rgb += lt.colour.rgb * atn * shaped * 0.01;
    }
    rgb += voxel_lights(id, dims);
    textureStore(light_next, vec3<i32>(id), vec4(to_srgb(max(rgb, vec3(0.0))), stored_alpha));
}
