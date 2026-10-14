// One step of the original's voxel global illumination, a light-propagation
// volume: each open voxel gathers its eight diagonal neighbours' light
// (weighted by how open they are), keeps most of it, gains a faint base
// ambient and the direct light of nearby lights. Light builds up over many
// steps. Storage is 8-bit sRGB like the original's. Rules:
// docs/RENDERING.md (Voxel GI).

struct GiLight {
    // World position (m), radius (m).
    position_radius: vec4<f32>,
    // Per-step injection colour (linear colour × intensity × 0.15 × 0.01).
    colour: vec4<f32>,
}

struct GiParams {
    // Lowest corner of the block (m); w = base ambient per step.
    origin: vec4<f32>,
    // Size of the block (m); w = number of lights.
    extent: vec4<f32>,
    lights: array<GiLight, 32>,
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
        let d = length(world - lt.position_radius.xyz);
        let atn = pow(max(1.0 - d / max(lt.position_radius.w, 1e-3), 0.0), 4.0) * open;
        rgb += lt.colour.rgb * atn;
    }
    textureStore(light_next, vec3<i32>(id), vec4(to_srgb(max(rgb, vec3(0.0))), stored_alpha));
}
