// Terrain layering: up to four palette materials per tile, stacked in order.
// The vertex colour holds each layer's "over" opacity (the first layer is
// opaque); a layer with a blend mask sharpens its opacity against the mask so
// stones and grass blades show through in their own shapes. Shaded like the
// original's terrain program: its lighting (`exa_render::shading`) with a
// gloss taken from the ground's colour.

import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    mesh_bindings::mesh,
    mesh_view_bindings as view_bindings,
    mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT,
};
import exa_render::shading::{
    Surface, direct, schlick, gi_sample, in_gi, gi_ambient,
    REFLECTION_STRENGTH, ENVIRONMENT_FALLBACK, AMBIENT_SCALE,
};

struct TerrainSplat {
    // Linear RGB multiplier per layer; alpha is its cover strength.
    colour: array<vec4<f32>, 4>,
    // Mask sharpness s, t; z is 1 when the layer has a blend mask.
    blend: array<vec4<f32>, 4>,
    // Normal-pass mask sharpness s, t; normal-map strength; gloss.
    normal: array<vec4<f32>, 4>,
    // Texture repeats per metre per layer.
    tiling: vec4<f32>,
    gi_origin: vec4<f32>,
    gi_extent: vec4<f32>,
    flags: u32,
    pad0: u32,
    pad1: u32,
    pad2: u32,
}

const GI: u32 = 1u;
const ENVIRONMENT: u32 = 2u;

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> splat: TerrainSplat;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var diffuse0: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var diffuse0_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var diffuse1: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var diffuse1_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(105) var diffuse2: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(106) var diffuse2_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(107) var diffuse3: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(108) var diffuse3_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(109) var mask1: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(110) var mask1_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(111) var mask2: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(112) var mask2_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(113) var mask3: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(114) var mask3_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(115) var normal0: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(116) var normal0_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(117) var normal1: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(118) var normal1_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(119) var normal2: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(120) var normal2_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(121) var normal3: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(122) var normal3_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(123) var gi_volume: texture_3d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(124) var gi_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(125) var environment_map: texture_cube<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(126) var environment_sampler: sampler;

fn has(bit: u32) -> bool {
    return (splat.flags & bit) != 0u;
}

// Opacity of layer `k` from its over opacity `a` and mask value `m`, with
// sharpness `s`, `t` (unmasked layers keep `a`).
fn sharpen(a: f32, m: f32, s: f32, t: f32, masked: bool) -> f32 {
    if !masked {
        return a;
    }
    return clamp((s + t) * a - s * (1.0 - m), 0.0, 1.0);
}

// Colour (or gloss) of layer `k` stacked on `below`.
fn stack(below: vec3<f32>, c: vec3<f32>, a: f32, m: f32, k: i32) -> vec3<f32> {
    let b = splat.blend[k];
    if b.z < 0.5 {
        return mix(below, c, a);
    }
    let sharp = sharpen(a, m, b.x, b.y, true);
    return below * clamp(1.0 - sharp * splat.colour[k].a, 0.0, 1.0) + c * sharp;
}

// Tangent-space normal xy of layer `k` from its normal-map sample.
fn normal_xy(sample: vec4<f32>, k: i32) -> vec2<f32> {
    return (sample.xy * 2.0 - 1.0) * splat.normal[k].z;
}

// Normal xy of layer `k` stacked on `below`.
fn stack_normal(below: vec2<f32>, n: vec2<f32>, a: f32, m: f32, k: i32) -> vec2<f32> {
    let nb = splat.normal[k];
    return mix(below, n, sharpen(a, m, nb.x, nb.y, splat.blend[k].z > 0.5));
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var alpha = vec4<f32>(1.0, 0.0, 0.0, 0.0);
@if(VERTEX_COLORS) {
    alpha = in.color;
}

    let p = in.world_position.xz;
    let uv0 = p * splat.tiling.x;
    let uv1 = p * splat.tiling.y;
    let uv2 = p * splat.tiling.z;
    let uv3 = p * splat.tiling.w;
    let m1 = textureSample(mask1, mask1_sampler, uv1).r;
    let m2 = textureSample(mask2, mask2_sampler, uv2).r;
    let m3 = textureSample(mask3, mask3_sampler, uv3).r;

    var colour = textureSample(diffuse0, diffuse0_sampler, uv0).rgb * splat.colour[0].rgb;
    colour = stack(colour, textureSample(diffuse1, diffuse1_sampler, uv1).rgb * splat.colour[1].rgb, alpha.y, m1, 1);
    colour = stack(colour, textureSample(diffuse2, diffuse2_sampler, uv2).rgb * splat.colour[2].rgb, alpha.z, m2, 2);
    colour = stack(colour, textureSample(diffuse3, diffuse3_sampler, uv3).rgb * splat.colour[3].rgb, alpha.w, m3, 3);

    var gloss = vec3<f32>(splat.normal[0].w);
    gloss = stack(gloss, vec3<f32>(splat.normal[1].w), alpha.y, m1, 1);
    gloss = stack(gloss, vec3<f32>(splat.normal[2].w), alpha.z, m2, 2);
    gloss = stack(gloss, vec3<f32>(splat.normal[3].w), alpha.w, m3, 3);

    var nxy = normal_xy(textureSample(normal0, normal0_sampler, uv0), 0);
    nxy = stack_normal(nxy, normal_xy(textureSample(normal1, normal1_sampler, uv1), 1), alpha.y, m1, 1);
    nxy = stack_normal(nxy, normal_xy(textureSample(normal2, normal2_sampler, uv2), 2), alpha.z, m2, 2);
    nxy = stack_normal(nxy, normal_xy(textureSample(normal3, normal3_sampler, uv3), 3), alpha.w, m3, 3);
    let nz = sqrt(1.0 - min(dot(nxy, nxy), 0.999));
    // Texture u runs along +X and v along +Z on the ground.
    var ng = normalize(in.world_normal);
    if !is_front {
        ng = -ng;
    }
    let t = normalize(vec3<f32>(1.0, 0.0, 0.0) - ng * ng.x);
    let b = normalize(vec3<f32>(0.0, 0.0, 1.0) - ng * ng.z);
    let n = normalize(t * nxy.x + b * nxy.y + ng * nz);

    // The original's terrain gloss: bluish, darker ground shines, scaled by
    // the material's glossiness.
    let shine = sqrt(min(max(colour.b - colour.r * 0.8 + 0.01, 0.0) * clamp(gloss.x, 0.0, 1.0) * 14.0, 0.9));

    let wp = in.world_position.xyz;
    let v = normalize(view_bindings::view.world_position.xyz - wp);
    var s: Surface;
    s.n = n;
    s.v = v;
    s.ndv = abs(dot(v, n));
    s.rough = clamp(1.0 - shine, 0.02, 1.0);
    s.f0 = vec3(0.04);
    s.legacy = false;

    let receiver = (mesh[in.instance_index].flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u;
    let lit = direct(s, in.world_position, n, in.position.xy, receiver);

    let level = view_bindings::lights.ambient_color.rgb;
    var ambient = level * ENVIRONMENT_FALLBACK;
    var mirrored = vec3(ENVIRONMENT_FALLBACK);
    let r = reflect(-v, n);
    let k = (s.rough + 1.0) * (s.rough + 1.0) / 8.0;
    let gv = s.ndv / (s.ndv * (1.0 - k) + k);
    var reflected: vec3<f32>;
    let gi_origin = splat.gi_origin.xyz;
    let gi_extent = splat.gi_extent.xyz;
    if has(GI) && in_gi(gi_origin, gi_extent, wp) {
        ambient = gi_ambient(gi_sample(gi_volume, gi_sampler, gi_origin, gi_extent, wp, n), level);
        if has(ENVIRONMENT) {
            mirrored = textureSampleLevel(environment_map, environment_sampler, r, max(s.rough - 0.05, 0.0) * 8.0).rgb;
        }
        reflected = mirrored * ambient * REFLECTION_STRENGTH * 2.0 * gv;
    } else {
        if has(ENVIRONMENT) {
            ambient = textureSampleLevel(environment_map, environment_sampler, n, 5.0).rgb * level;
            mirrored = textureSampleLevel(environment_map, environment_sampler, r, max(s.rough - 0.1, 0.0) * 8.0).rgb;
        }
        reflected = (mirrored + mirrored * mirrored) * level / AMBIENT_SCALE * REFLECTION_STRENGTH * gv;
    }
    var out: FragmentOutput;
    let specular = lit.specular + reflected * schlick(s.f0, s.ndv);
    out.color = vec4((lit.diffuse + ambient) * colour + specular * min(colour.r * 255.0, 1.0), 1.0);
    return out;
}
