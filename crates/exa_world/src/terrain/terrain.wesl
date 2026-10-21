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

// Up to 16 layers: every palette material the tile uses, in palette order.
const LAYERS: u32 = 16u;

struct TerrainSplat {
    // Linear RGB multiplier per layer; alpha is its cover strength.
    colour: array<vec4<f32>, 16>,
    // Mask sharpness s, t; z is 1 when the layer has a blend mask; w its
    // texture repeats per metre.
    blend: array<vec4<f32>, 16>,
    // Normal-pass mask sharpness s, t; normal-map strength; gloss.
    normal: array<vec4<f32>, 16>,
    // Gloss factor; parallax height base, reference, scale.
    surface: array<vec4<f32>, 16>,
    gi_origin: vec4<f32>,
    gi_extent: vec4<f32>,
    flags: u32,
    // Layers in use.
    layers: u32,
    pad1: u32,
    pad2: u32,
}

const GI: u32 = 1u;
const ENVIRONMENT: u32 = 2u;
const DEBUG_GI: u32 = 4u;
const SPLIT_TAPS: u32 = 8u;

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> splat: TerrainSplat;
// Every layer texture is read through one repeating sampler.
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var layer_sampler: sampler;
// The layers' over opacities, one texel per vertex: layer k in channel
// k % 4 of array layer k / 4.
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var cover_map: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var cover_sampler: sampler;
// Per layer: diffuse; parallax height (r), gloss (g), blend mask (b); normal.
@group(constants::MATERIAL_BIND_GROUP) @binding(140) var diffuse0: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(141) var diffuse1: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(142) var diffuse2: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(143) var diffuse3: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(144) var diffuse4: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(145) var diffuse5: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(146) var diffuse6: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(147) var diffuse7: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(148) var diffuse8: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(149) var diffuse9: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(150) var diffuse10: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(151) var diffuse11: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(152) var diffuse12: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(153) var diffuse13: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(154) var diffuse14: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(155) var diffuse15: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(160) var surface0: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(161) var surface1: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(162) var surface2: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(163) var surface3: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(164) var surface4: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(165) var surface5: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(166) var surface6: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(167) var surface7: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(168) var surface8: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(169) var surface9: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(170) var surface10: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(171) var surface11: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(172) var surface12: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(173) var surface13: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(174) var surface14: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(175) var surface15: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(180) var normal0: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(181) var normal1: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(182) var normal2: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(183) var normal3: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(184) var normal4: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(185) var normal5: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(186) var normal6: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(187) var normal7: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(188) var normal8: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(189) var normal9: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(190) var normal10: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(191) var normal11: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(192) var normal12: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(193) var normal13: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(194) var normal14: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(195) var normal15: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(123) var gi_volume: texture_3d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(124) var gi_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(125) var environment_map: texture_cube<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(126) var environment_sampler: sampler;

fn has(bit: u32) -> bool {
    return (splat.flags & bit) != 0u;
}

// Layer `k`'s textures at `uv` (base level).
fn layer_diffuse(k: u32, uv: vec2<f32>) -> vec4<f32> {
    switch k {
        case 0u: { return textureSampleLevel(diffuse0, layer_sampler, uv, 0.0); }
        case 1u: { return textureSampleLevel(diffuse1, layer_sampler, uv, 0.0); }
        case 2u: { return textureSampleLevel(diffuse2, layer_sampler, uv, 0.0); }
        case 3u: { return textureSampleLevel(diffuse3, layer_sampler, uv, 0.0); }
        case 4u: { return textureSampleLevel(diffuse4, layer_sampler, uv, 0.0); }
        case 5u: { return textureSampleLevel(diffuse5, layer_sampler, uv, 0.0); }
        case 6u: { return textureSampleLevel(diffuse6, layer_sampler, uv, 0.0); }
        case 7u: { return textureSampleLevel(diffuse7, layer_sampler, uv, 0.0); }
        case 8u: { return textureSampleLevel(diffuse8, layer_sampler, uv, 0.0); }
        case 9u: { return textureSampleLevel(diffuse9, layer_sampler, uv, 0.0); }
        case 10u: { return textureSampleLevel(diffuse10, layer_sampler, uv, 0.0); }
        case 11u: { return textureSampleLevel(diffuse11, layer_sampler, uv, 0.0); }
        case 12u: { return textureSampleLevel(diffuse12, layer_sampler, uv, 0.0); }
        case 13u: { return textureSampleLevel(diffuse13, layer_sampler, uv, 0.0); }
        case 14u: { return textureSampleLevel(diffuse14, layer_sampler, uv, 0.0); }
        case 15u: { return textureSampleLevel(diffuse15, layer_sampler, uv, 0.0); }
        default: { return vec4(0.0); }
    }
}

fn layer_surface(k: u32, uv: vec2<f32>) -> vec4<f32> {
    switch k {
        case 0u: { return textureSampleLevel(surface0, layer_sampler, uv, 0.0); }
        case 1u: { return textureSampleLevel(surface1, layer_sampler, uv, 0.0); }
        case 2u: { return textureSampleLevel(surface2, layer_sampler, uv, 0.0); }
        case 3u: { return textureSampleLevel(surface3, layer_sampler, uv, 0.0); }
        case 4u: { return textureSampleLevel(surface4, layer_sampler, uv, 0.0); }
        case 5u: { return textureSampleLevel(surface5, layer_sampler, uv, 0.0); }
        case 6u: { return textureSampleLevel(surface6, layer_sampler, uv, 0.0); }
        case 7u: { return textureSampleLevel(surface7, layer_sampler, uv, 0.0); }
        case 8u: { return textureSampleLevel(surface8, layer_sampler, uv, 0.0); }
        case 9u: { return textureSampleLevel(surface9, layer_sampler, uv, 0.0); }
        case 10u: { return textureSampleLevel(surface10, layer_sampler, uv, 0.0); }
        case 11u: { return textureSampleLevel(surface11, layer_sampler, uv, 0.0); }
        case 12u: { return textureSampleLevel(surface12, layer_sampler, uv, 0.0); }
        case 13u: { return textureSampleLevel(surface13, layer_sampler, uv, 0.0); }
        case 14u: { return textureSampleLevel(surface14, layer_sampler, uv, 0.0); }
        case 15u: { return textureSampleLevel(surface15, layer_sampler, uv, 0.0); }
        default: { return vec4(0.5, 0.0, 1.0, 1.0); }
    }
}

fn layer_normal(k: u32, uv: vec2<f32>) -> vec4<f32> {
    switch k {
        case 0u: { return textureSampleLevel(normal0, layer_sampler, uv, 0.0); }
        case 1u: { return textureSampleLevel(normal1, layer_sampler, uv, 0.0); }
        case 2u: { return textureSampleLevel(normal2, layer_sampler, uv, 0.0); }
        case 3u: { return textureSampleLevel(normal3, layer_sampler, uv, 0.0); }
        case 4u: { return textureSampleLevel(normal4, layer_sampler, uv, 0.0); }
        case 5u: { return textureSampleLevel(normal5, layer_sampler, uv, 0.0); }
        case 6u: { return textureSampleLevel(normal6, layer_sampler, uv, 0.0); }
        case 7u: { return textureSampleLevel(normal7, layer_sampler, uv, 0.0); }
        case 8u: { return textureSampleLevel(normal8, layer_sampler, uv, 0.0); }
        case 9u: { return textureSampleLevel(normal9, layer_sampler, uv, 0.0); }
        case 10u: { return textureSampleLevel(normal10, layer_sampler, uv, 0.0); }
        case 11u: { return textureSampleLevel(normal11, layer_sampler, uv, 0.0); }
        case 12u: { return textureSampleLevel(normal12, layer_sampler, uv, 0.0); }
        case 13u: { return textureSampleLevel(normal13, layer_sampler, uv, 0.0); }
        case 14u: { return textureSampleLevel(normal14, layer_sampler, uv, 0.0); }
        case 15u: { return textureSampleLevel(normal15, layer_sampler, uv, 0.0); }
        default: { return vec4(0.5, 0.5, 1.0, 1.0); }
    }
}

// The layers' over opacities at tile coordinates `t` (0–1 across the
// tile's vertices).
struct Covers {
    a: array<f32, 16>,
}

fn covers(t: vec2<f32>) -> Covers {
    // Vertex i's texel centre: (i + ½) / 241.
    let n = f32(textureDimensions(cover_map).x);
    let uv = (t * (n - 1.0) + 0.5) / n;
    var out: Covers;
    for (var l = 0u; l < LAYERS / 4u; l++) {
        let c = textureSampleLevel(cover_map, cover_sampler, uv, l, 0.0);
        out.a[4u * l] = c.x;
        out.a[4u * l + 1u] = c.y;
        out.a[4u * l + 2u] = c.z;
        out.a[4u * l + 3u] = c.w;
    }
    out.a[0] = 1.0;
    return out;
}

// Whether layer `k` covers anything here (the first always does).
fn covered(c: Covers, k: u32) -> bool {
    return k == 0u || c.a[k] > 0.0;
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
fn stack(below: vec3<f32>, c: vec3<f32>, a: f32, m: f32, k: u32) -> vec3<f32> {
    let b = splat.blend[k];
    if b.z < 0.5 {
        return mix(below, c, a);
    }
    let sharp = sharpen(a, m, b.x, b.y, true);
    return below * clamp(1.0 - sharp * splat.colour[k].a, 0.0, 1.0) + c * sharp;
}

// Tangent-space normal xy of layer `k` from its normal-map sample.
fn normal_xy(sample: vec4<f32>, k: u32) -> vec2<f32> {
    return (sample.xy * 2.0 - 1.0) * splat.normal[k].z;
}

// Layer `k`'s texture coordinates at ground position `p` (metres along +X,
// −Z).
fn layer_uv(p: vec2<f32>, k: u32) -> vec2<f32> {
    return p * splat.blend[k].w;
}

// The composited parallax height at `p`: each layer's f20 + (B − f21)·f22,
// stacked like the normals.
fn height_at(p: vec2<f32>, c: Covers) -> f32 {
    var h = 0.0;
    for (var k = 0u; k < splat.layers; k++) {
        if !covered(c, k) {
            continue;
        }
        let su = layer_surface(k, layer_uv(p, k));
        let q = splat.surface[k];
        let lh = q.y + (su.r - q.z) * q.w;
        if k == 0u {
            h = lh;
        } else {
            let nb = splat.normal[k];
            h = mix(h, lh, sharpen(c.a[k], su.b, nb.x, nb.y, splat.blend[k].z > 0.5));
        }
    }
    return h;
}

// Terrain blocks' shadow-bias level (see `exa_render::shading`).
const TERRAIN_SHADOW_BIAS: u32 = 3u;
// The original's terrain diffuse factor (`MatDif`).
const TERRAIN_DIFFUSE: f32 = 0.75;
// Parallax scale in 6 m block units.
const PARALLAX: f32 = 0.007;
const BLOCK: f32 = 6.0;

// Normal xy of layer `k` stacked on `below`.
fn stack_normal(below: vec2<f32>, n: vec2<f32>, a: f32, m: f32, k: u32) -> vec2<f32> {
    let nb = splat.normal[k];
    return mix(below, n, sharpen(a, m, nb.x, nb.y, splat.blend[k].z > 0.5));
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var tile_uv = vec2<f32>(0.0);
@if(VERTEX_UVS_A) {
    tile_uv = in.uv;
}
    let cover = covers(tile_uv);
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
    let d = height_at(start * BLOCK, cover) - 0.5;
    var uvb = start - dsp * d;
    var last = d;
    for (var k = 0; k < 3; k++) {
        last = height_at(uvb * BLOCK, cover) - 0.5;
        uvb -= dsp * last;
    }
    uvb = mix(uvb, start, min((d - last) * 2.0, 1.0));
    let p = uvb * BLOCK;

    // The original composites each 6 m block into textures of about 170
    // texels per metre without mip levels, so terrain always shows its
    // layers' full detail: every layer is read at its base level
    // (supersampling smooths what that aliases). Colour, gloss and normal
    // stack in palette order; layers not covering this point are skipped.
    var colour = vec3<f32>(0.0);
    var gloss = vec3<f32>(0.0);
    var nxy = vec2<f32>(0.0);
    for (var k = 0u; k < splat.layers; k++) {
        if !covered(cover, k) {
            continue;
        }
        let uv = layer_uv(p, k);
        let su = layer_surface(k, uv);
        let c = layer_diffuse(k, uv).rgb * splat.colour[k].rgb;
        let g = vec3<f32>(su.g * splat.surface[k].x);
        let nk = normal_xy(layer_normal(k, uv), k);
        if k == 0u {
            colour = c;
            gloss = g;
            nxy = nk;
        } else {
            let a = cover.a[k];
            colour = stack(colour, c, a, su.b, k);
            // The composited gloss map: each layer's gloss × its factor,
            // stacked like the colour; roughness is 1 − gloss (no wetness
            // indoors).
            gloss = stack(gloss, g, a, su.b, k);
            nxy = stack_normal(nxy, nk, a, su.b, k);
        }
    }
    // The specular term is masked by the composite colour, before the
    // terrain's diffuse factor, as the original's program does.
    let specular_mask = min(colour.r * 255.0, 1.0);
    colour *= TERRAIN_DIFFUSE;
    let nz = sqrt(1.0 - min(dot(nxy, nxy), 0.999));
    // The normal map's frame, as the original's lit program builds it: +X
    // projected onto the surface, then its cross with the normal (on flat
    // ground +X and +Z; green along −v), so the frame stays orthonormal on
    // a slope. `terrain::frame` is the tested model of it.
    let t = normalize(vec3<f32>(1.0, 0.0, 0.0) - ng * ng.x);
    let b = normalize(cross(t, ng));
    let n = normalize(t * nxy.x + b * nxy.y + ng * nz);

    var s: Surface;
    s.n = n;
    s.v = v;
    s.ndv = abs(dot(v, n));
    // The composite gloss is an 8-bit value; roughness is 1 - it, with no
    // other floor (the model program's 0.02 is Exarust's). Full gloss is a
    // mirror without a highlight, as in the original; the shared light
    // keeps its one singular point at 0.
    s.rough = 1.0 - clamp(gloss.x, 0.0, 1.0);
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
    out.color = vec4((lit.diffuse + ambient) * colour + specular * specular_mask, 1.0);
    return out;
}
