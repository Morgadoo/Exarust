// One step of the original's voxel global illumination, a light-propagation
// volume: each open voxel gathers its eight diagonal neighbours' light
// (weighted by how open they are), keeps most of it, gains a faint base
// ambient and the direct light of its sector's lights; voxel lights (glowing
// items) then add their light straight into the voxels around them. Light
// builds up over many steps. Storage is 8-bit sRGB like the original's.
// Rules: docs/RENDERING.md (Voxel GI).

struct GiLight {
    // World position (m), radius (m).
    position_radius: vec4<f32>,
    // Per-step injection colour (linear colour × intensity × 0.15 × 0.01).
    colour: vec4<f32>,
    // Vertical shaping y, z (1, 1 for none).
    shaping: vec4<f32>,
}

struct GiVoxelLight {
    // The light in voxels from the block's corner (texture axes).
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

@group(0) @binding(0) var light_prev: texture_3d<f32>;
@group(0) @binding(1) var occupancy: texture_3d<f32>;
@group(0) @binding(2) var linear_sampler: sampler;
@group(0) @binding(3) var light_next: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(4) var<uniform> params: GiParams;

fn to_srgb(c: vec3<f32>) -> vec3<f32> {
    let l = clamp(c, vec3(0.0), vec3(1.0));
    return select(1.055 * pow(l, vec3(1.0 / 2.4)) - 0.055, l * 12.92, l <= vec3(0.0031308));
}

fn openness(occ: f32) -> f32 {
    return max(1.0 - 13.0 * occ, 0.0);
}

// What the voxel lights add to voxel `id` this step (the original's
// gi_emission pass, one light at a time). Each reaches the box the CPU
// worked out for it and falls off as max(1 − 5000·d², 0)⁴, with d in
// texture units (the vertical counted in voxels across, W = 256), nothing
// beyond 3.62 voxels. Geometry at the voxel or halfway to the light
// (openness 1 − occupancy, unlike the propagation's) leaves only the
// voxels within about 1.2 of the light lit.
fn voxel_lights(id: vec3<u32>, dims: vec3<u32>) -> vec3<f32> {
    var rgb = vec3(0.0);
    let size = vec3<f32>(dims);
    let centre = vec3<f32>(id) + 0.5;
    for (var i = 0u; i < params.voxel_count.x; i++) {
        let lt = params.voxel_lights[i];
        if any(id < lt.lo.xyz) || any(id >= lt.hi.xyz) {
            continue;
        }
        let to = (centre - lt.position.xyz) / vec3(size.x, size.y, size.x);
        let atn = pow(max(1.0 - dot(to, to) * 5000.0, 0.0), 4.0);
        if atn <= 0.0 {
            continue;
        }
        let here = centre / size;
        let mid = mix(lt.position.xyz / size, here, 0.5);
        let open_here = 1.0 - textureSampleLevel(occupancy, linear_sampler, here, 0.0).r;
        let open_mid = 1.0 - textureSampleLevel(occupancy, linear_sampler, mid, 0.0).r;
        let blocked = min(20.0 * (1.0 - open_here * open_mid), 1.0);
        let near = max(2.0 - 0.75 / (atn * atn), 0.0);
        rgb += lt.colour.rgb * atn * 0.25 * (1.0 - blocked * (1.0 - near));
    }
    return rgb;
}

// The voxels the manager does not draw this frame: the original clears the
// scratch target to (0, 0, 0, 0) and then runs the voxel lights' additive
// pass over it, so they take the lights' colour and no openness.
fn write_dark(id: vec3<u32>, dims: vec3<u32>) {
    textureStore(light_next, vec3<i32>(id), vec4(to_srgb(voxel_lights(id, dims)), 0.0));
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
    let size = vec3<f32>(dims);
    let uvw = (vec3<f32>(id) + 0.5) / size;
    // The light is sampled through an sRGB view: linear, decoded before
    // filtering.
    let old = textureSampleLevel(light_prev, linear_sampler, uvw, 0.0).rgb;
    let occ0 = textureSampleLevel(occupancy, linear_sampler, uvw, 0.0).r;
    let stored_alpha = clamp(1.0 - 5.0 * occ0, 0.0, 1.0);
    var open = openness(occ0);

    var sum = vec3(0.0);
    var s = 0.0;
    let step = vec3(1.33) / size;
    for (var i = 0u; i < 8u; i++) {
        let corner = vec3<f32>(f32(i & 1u), f32((i >> 1u) & 1u), f32((i >> 2u) & 1u)) * 2.0 - 1.0;
        let n = uvw + corner * step;
        let a = openness(textureSampleLevel(occupancy, linear_sampler, n, 0.0).r);
        s += a;
        // Indoors (no sun) a neighbour's light counts by its openness squared.
        let p = textureSampleLevel(light_prev, linear_sampler, n, 0.0).rgb * a * a;
        sum += mix(old, p, a);
    }
    // The lowest slices fade out.
    open *= 1.0 - exp2(-16.0 * uvw.z);
    let k = min(8.0 * s, 1.0) * (0.3 * open + 0.699);
    var rgb = sum * k * 0.125 + params.base.rgb * open;

    // Direct light, from the lights of this sector's group only. Texture
    // axes are world X, Z, Y.
    let world = params.origin.xyz + vec3(uvw.x, uvw.z, uvw.y) * params.extent.xyz;
    for (var i = 0u; i < plan.count; i++) {
        let lt = params.lights[plan.first + i];
        let to = lt.position_radius.xyz - world;
        let d = length(to);
        // The light's vertical shaping, as on surfaces.
        let ly = to.y / max(d, 1e-4);
        let z = lt.shaping.y;
        let shape = max((exp2((ly + (1.0 - ly) * lt.shaping.x) * z) - 1.0) / max(exp2(z) - 1.0, 1e-4), 0.0);
        let atn = pow(max(1.0 - d / max(lt.position_radius.w, 1e-3), 0.0), 4.0) * open * shape;
        rgb += lt.colour.rgb * atn;
    }
    rgb += voxel_lights(id, dims);
    textureStore(light_next, vec3<i32>(id), vec4(to_srgb(max(rgb, vec3(0.0))), stored_alpha));
}
