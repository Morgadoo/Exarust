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
    Surface, taps, direct, schlick, gi_sample, in_gi, gi_ambient,
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
    // Gloss factor; parallax height base, reference, scale.
    surface: array<vec4<f32>, 4>,
    gi_origin: vec4<f32>,
    gi_extent: vec4<f32>,
    flags: u32,
    pad0: u32,
    pad1: u32,
    pad2: u32,
}

const GI: u32 = 1u;
const ENVIRONMENT: u32 = 2u;
const DEBUG_GI: u32 = 4u;
const SPLIT_TAPS: u32 = 8u;

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> splat: TerrainSplat;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var diffuse0: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var diffuse0_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var diffuse1: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var diffuse1_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(105) var diffuse2: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(106) var diffuse2_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(107) var diffuse3: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(108) var diffuse3_sampler: sampler;
// Per layer: parallax height (r), gloss (g), blend mask (b).
@group(constants::MATERIAL_BIND_GROUP) @binding(109) var surface0: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(110) var surface0_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(111) var surface1: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(112) var surface1_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(113) var surface2: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(114) var surface2_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(127) var surface3: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(128) var surface3_sampler: sampler;
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

// The four layers' packed surface samples at ground position `p` (metres
// along +X, −Z).
struct Surfaces {
    s: array<vec4<f32>, 4>,
}

fn surfaces(p: vec2<f32>) -> Surfaces {
    var out: Surfaces;
    out.s[0] = textureSampleLevel(surface0, surface0_sampler, p * splat.tiling.x, 0.0);
    out.s[1] = textureSampleLevel(surface1, surface1_sampler, p * splat.tiling.y, 0.0);
    out.s[2] = textureSampleLevel(surface2, surface2_sampler, p * splat.tiling.z, 0.0);
    out.s[3] = textureSampleLevel(surface3, surface3_sampler, p * splat.tiling.w, 0.0);
    return out;
}

// The composited parallax height at `p`: each layer's f20 + (B − f21)·f22,
// stacked like the normals.
fn height_at(p: vec2<f32>, alpha: vec4<f32>) -> f32 {
    let su = surfaces(p);
    var h = layer_height(su.s[0], 0);
    h = stack_height(h, layer_height(su.s[1], 1), alpha.y, su.s[1].b, 1);
    h = stack_height(h, layer_height(su.s[2], 2), alpha.z, su.s[2].b, 2);
    h = stack_height(h, layer_height(su.s[3], 3), alpha.w, su.s[3].b, 3);
    return h;
}

fn layer_height(su: vec4<f32>, k: i32) -> f32 {
    let q = splat.surface[k];
    return q.y + (su.r - q.z) * q.w;
}

fn stack_height(below: f32, h: f32, a: f32, m: f32, k: i32) -> f32 {
    let nb = splat.normal[k];
    return mix(below, h, sharpen(a, m, nb.x, nb.y, splat.blend[k].z > 0.5));
}

// Terrain blocks' shadow-bias level (see `exa_render::shading`).
const TERRAIN_SHADOW_BIAS: u32 = 3u;
// The original's terrain diffuse factor (`MatDif`).
const TERRAIN_DIFFUSE: f32 = 0.75;
// Parallax scale in 6 m block units.
const PARALLAX: f32 = 0.007;
const BLOCK: f32 = 6.0;

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
    let wp = in.world_position.xyz;
    let v = normalize(view_bindings::view.world_position.xyz - wp);
    var ng = normalize(in.world_normal);
    if !is_front {
        ng = -ng;
    }

    // Texture u runs along +X and v along −Z on the ground, as the original
    // maps its terrain (and its tile-grid floors).
    let ground = vec2(wp.x, -wp.z);
    // Parallax in 6 m block coordinates, as the original's terrain program:
    // the tangent-space view shifts them by the composited height in four
    // accumulating steps, then eases back where the height changes sharply.
    let tu = normalize(vec3(1.0, 0.0, 0.0) - ng * ng.x);
    let tv = normalize(vec3(0.0, 0.0, -1.0) + ng * ng.z);
    let eye = -v;
    let vt = vec3(dot(eye, tu), dot(eye, tv), dot(eye, ng));
    let dsp = vt.xy * PARALLAX * (4.0 + 3.0 * vt.z);
    let start = ground / BLOCK;
    let d = height_at(start * BLOCK, alpha) - 0.5;
    var uvb = start - dsp * d;
    var last = d;
    for (var k = 0; k < 3; k++) {
        last = height_at(uvb * BLOCK, alpha) - 0.5;
        uvb -= dsp * last;
    }
    uvb = mix(uvb, start, min((d - last) * 2.0, 1.0));
    let p = uvb * BLOCK;

    let uv0 = p * splat.tiling.x;
    let uv1 = p * splat.tiling.y;
    let uv2 = p * splat.tiling.z;
    let uv3 = p * splat.tiling.w;
    let su = surfaces(p);
    let m1 = su.s[1].b;
    let m2 = su.s[2].b;
    let m3 = su.s[3].b;

    // The original composites each 6 m block into textures of about 170
    // texels per metre without mip levels, so terrain always shows its
    // layers' full detail: colour and normal layers are read at their base
    // level (supersampling smooths what that aliases).
    var colour = textureSampleLevel(diffuse0, diffuse0_sampler, uv0, 0.0).rgb * splat.colour[0].rgb;
    colour = stack(colour, textureSampleLevel(diffuse1, diffuse1_sampler, uv1, 0.0).rgb * splat.colour[1].rgb, alpha.y, m1, 1);
    colour = stack(colour, textureSampleLevel(diffuse2, diffuse2_sampler, uv2, 0.0).rgb * splat.colour[2].rgb, alpha.z, m2, 2);
    colour = stack(colour, textureSampleLevel(diffuse3, diffuse3_sampler, uv3, 0.0).rgb * splat.colour[3].rgb, alpha.w, m3, 3);
    colour *= TERRAIN_DIFFUSE;

    // The composited gloss map: each layer's gloss × its factor, stacked like
    // the colour; roughness is 1 − gloss (no wetness indoors).
    var gloss = vec3<f32>(su.s[0].g * splat.surface[0].x);
    gloss = stack(gloss, vec3<f32>(su.s[1].g * splat.surface[1].x), alpha.y, m1, 1);
    gloss = stack(gloss, vec3<f32>(su.s[2].g * splat.surface[2].x), alpha.z, m2, 2);
    gloss = stack(gloss, vec3<f32>(su.s[3].g * splat.surface[3].x), alpha.w, m3, 3);

    var nxy = normal_xy(textureSampleLevel(normal0, normal0_sampler, uv0, 0.0), 0);
    nxy = stack_normal(nxy, normal_xy(textureSampleLevel(normal1, normal1_sampler, uv1, 0.0), 1), alpha.y, m1, 1);
    nxy = stack_normal(nxy, normal_xy(textureSampleLevel(normal2, normal2_sampler, uv2, 0.0), 2), alpha.z, m2, 2);
    nxy = stack_normal(nxy, normal_xy(textureSampleLevel(normal3, normal3_sampler, uv3, 0.0), 3), alpha.w, m3, 3);
    let nz = sqrt(1.0 - min(dot(nxy, nxy), 0.999));
    // The normal map's frame: +X and +Z across the ground (green along −v),
    // as the original's terrain program builds it.
    let t = normalize(vec3<f32>(1.0, 0.0, 0.0) - ng * ng.x);
    let b = normalize(vec3<f32>(0.0, 0.0, 1.0) - ng * ng.z);
    let n = normalize(t * nxy.x + b * nxy.y + ng * nz);

    var s: Surface;
    s.n = n;
    s.v = v;
    s.ndv = abs(dot(v, n));
    s.rough = clamp(1.0 - clamp(gloss.x, 0.0, 1.0), 0.02, 1.0);
    s.f0 = vec3(0.04);
    s.legacy = false;

    let receiver = (mesh[in.instance_index].flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u;
    let disc = taps(in.position.xy, has(SPLIT_TAPS));
    let lit = direct(s, in.world_position, n, in.position.xy, disc, receiver, TERRAIN_SHADOW_BIAS);

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
        let g = gi_sample(gi_volume, gi_sampler, gi_origin, gi_extent, wp, n, disc);
        if has(DEBUG_GI) {
            var dbg: FragmentOutput;
            dbg.color = vec4(g.rgb * 10.0, 1.0);
            return dbg;
        }
        ambient = gi_ambient(g, level);
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
