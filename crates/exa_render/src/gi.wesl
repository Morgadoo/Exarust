// One step of the original's voxel global illumination, a light-propagation
// volume: each open voxel gathers its eight diagonal neighbours' light
// (weighted by how open they are), keeps most of it, gains a faint base
// ambient and the direct light of nearby lights; voxel lights (glowing
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

struct GiParams {
    // Lowest corner of the block (m); w = base ambient per step.
    origin: vec4<f32>,
    // Size of the block (m); w = number of lights.
    extent: vec4<f32>,
    lights: array<GiLight, 32>,
    // x = number of voxel lights.
    voxel_count: vec4<u32>,
    voxel_lights: array<GiVoxelLight, 64>,
}

@group(0) @binding(0) var light_prev: texture_3d<f32>;
@group(0) @binding(1) var occupancy: texture_3d<f32>;
@group(0) @binding(2) var linear_sampler: sampler;
@group(0) @binding(3) var light_next: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(4) var<uniform> params: GiParams;
// Per column, the lowest slice light propagates in (×255); 255 for none.
@group(0) @binding(5) var floors: texture_3d<f32>;

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

@compute @workgroup_size(4, 4, 4)
fn propagate(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(light_next);
    if any(id >= dims) {
        return;
    }
    // Slices below the sector's floor, the top slice and empty sectors are
    // never updated.
    let floor = u32(round(textureLoad(floors, vec3(id.xy, 0u), 0).r * 255.0));
    if floor == 255u || id.z < floor || id.z + 1u >= dims.z {
        let kept = textureLoad(light_prev, vec3<i32>(id), 0);
        textureStore(light_next, vec3<i32>(id), vec4(to_srgb(kept.rgb), kept.a));
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
    var rgb = sum * k * 0.125 + vec3(params.origin.w * open);

    // Direct light. Texture axes are world X, Z, Y.
    let world = params.origin.xyz + vec3(uvw.x, uvw.z, uvw.y) * params.extent.xyz;
    let count = u32(params.extent.w);
    for (var i = 0u; i < count; i++) {
        let lt = params.lights[i];
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
