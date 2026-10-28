// The terrain's composite bake: one 6 m block's layers stacked into its
// set's four targets, as the original's compositor driver writes them, in
// its pass order: the tangent-space normal (RG8), the colour (RGB8, here an
// sRGB RGBA8), the gloss (R8) and the half-size parallax height (R8). Each
// pass is a full-target quad over the block: texel (i + ½)/N of the target
// is the point (i + ½)/N across the block, so the draw reads the block's
// composite at its own coordinates. The stacking is the per-pixel terrain
// draw's (`terrain_layers`), evaluated once per texel instead of per pixel;
// the original's groups of four additive layers fold into one loop, as the
// per-pixel draw folds them. Rules: docs/RENDERING.md (Terrain).

import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;
import exa_render::terrain_layers::{sharpen, stack_colour, stack_xy, normal_xy, layer_height};

struct TerrainBakeParams {
    // Per layer, as the terrain draw's uniform holds them: linear RGB
    // multiplier and cover strength; mask sharpness s, t, masked flag and
    // repeats per metre; normal-pass sharpness s, t, normal strength and
    // gloss; gloss factor and the parallax height's base, reference, scale.
    colour: array<vec4<f32>, 16>,
    blend: array<vec4<f32>, 16>,
    normal: array<vec4<f32>, 16>,
    surface: array<vec4<f32>, 16>,
    // The block's low world corner x, z (metres) and its side (metres).
    block: vec4<f32>,
    // The block's low corner in the tile's vertex coordinates (0–1) and its
    // span in them.
    tile: vec4<f32>,
    layers: u32,
    pad1: u32,
    pad2: u32,
    pad3: u32,
}

@group(0) @binding(0) var<uniform> bake: TerrainBakeParams;
@group(0) @binding(1) var layer_sampler: sampler;
@group(0) @binding(2) var cover_map: texture_2d_array<f32>;
@group(0) @binding(3) var cover_sampler: sampler;
@group(0) @binding(4) var diffuse0: texture_2d<f32>;
@group(0) @binding(5) var diffuse1: texture_2d<f32>;
@group(0) @binding(6) var diffuse2: texture_2d<f32>;
@group(0) @binding(7) var diffuse3: texture_2d<f32>;
@group(0) @binding(8) var diffuse4: texture_2d<f32>;
@group(0) @binding(9) var diffuse5: texture_2d<f32>;
@group(0) @binding(10) var diffuse6: texture_2d<f32>;
@group(0) @binding(11) var diffuse7: texture_2d<f32>;
@group(0) @binding(12) var diffuse8: texture_2d<f32>;
@group(0) @binding(13) var diffuse9: texture_2d<f32>;
@group(0) @binding(14) var diffuse10: texture_2d<f32>;
@group(0) @binding(15) var diffuse11: texture_2d<f32>;
@group(0) @binding(16) var diffuse12: texture_2d<f32>;
@group(0) @binding(17) var diffuse13: texture_2d<f32>;
@group(0) @binding(18) var diffuse14: texture_2d<f32>;
@group(0) @binding(19) var diffuse15: texture_2d<f32>;
@group(0) @binding(20) var surface0: texture_2d<f32>;
@group(0) @binding(21) var surface1: texture_2d<f32>;
@group(0) @binding(22) var surface2: texture_2d<f32>;
@group(0) @binding(23) var surface3: texture_2d<f32>;
@group(0) @binding(24) var surface4: texture_2d<f32>;
@group(0) @binding(25) var surface5: texture_2d<f32>;
@group(0) @binding(26) var surface6: texture_2d<f32>;
@group(0) @binding(27) var surface7: texture_2d<f32>;
@group(0) @binding(28) var surface8: texture_2d<f32>;
@group(0) @binding(29) var surface9: texture_2d<f32>;
@group(0) @binding(30) var surface10: texture_2d<f32>;
@group(0) @binding(31) var surface11: texture_2d<f32>;
@group(0) @binding(32) var surface12: texture_2d<f32>;
@group(0) @binding(33) var surface13: texture_2d<f32>;
@group(0) @binding(34) var surface14: texture_2d<f32>;
@group(0) @binding(35) var surface15: texture_2d<f32>;
@group(0) @binding(36) var normal0: texture_2d<f32>;
@group(0) @binding(37) var normal1: texture_2d<f32>;
@group(0) @binding(38) var normal2: texture_2d<f32>;
@group(0) @binding(39) var normal3: texture_2d<f32>;
@group(0) @binding(40) var normal4: texture_2d<f32>;
@group(0) @binding(41) var normal5: texture_2d<f32>;
@group(0) @binding(42) var normal6: texture_2d<f32>;
@group(0) @binding(43) var normal7: texture_2d<f32>;
@group(0) @binding(44) var normal8: texture_2d<f32>;
@group(0) @binding(45) var normal9: texture_2d<f32>;
@group(0) @binding(46) var normal10: texture_2d<f32>;
@group(0) @binding(47) var normal11: texture_2d<f32>;
@group(0) @binding(48) var normal12: texture_2d<f32>;
@group(0) @binding(49) var normal13: texture_2d<f32>;
@group(0) @binding(50) var normal14: texture_2d<f32>;
@group(0) @binding(51) var normal15: texture_2d<f32>;

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

// The layers' over opacities at the block's texel `uv`, read between the
// tile's vertices like the draw reads them (vertex i's texel centre is
// (i + ½)/241); the first layer is opaque.
struct Covers {
    a: array<f32, 16>,
}

fn covers(uv: vec2<f32>) -> Covers {
    let t = bake.tile.xy + uv * bake.tile.z;
    let n = f32(textureDimensions(cover_map).x);
    let c_uv = (t * (n - 1.0) + 0.5) / n;
    var out: Covers;
    for (var l = 0u; l < 4u; l++) {
        let c = textureSampleLevel(cover_map, cover_sampler, c_uv, l, 0.0);
        out.a[4u * l] = c.x;
        out.a[4u * l + 1u] = c.y;
        out.a[4u * l + 2u] = c.z;
        out.a[4u * l + 3u] = c.w;
    }
    out.a[0] = 1.0;
    return out;
}

fn covered(c: Covers, k: u32) -> bool {
    return k == 0u || c.a[k] > 0.0;
}

// Layer `k`'s texture coordinates at the block's texel `uv`: the ground
// point (metres along +X and −Z, as the draw maps the terrain) times the
// layer's repeats.
fn layer_uv(uv: vec2<f32>, k: u32) -> vec2<f32> {
    let x = bake.block.x + uv.x * bake.block.z;
    let z = bake.block.y + uv.y * bake.block.z;
    return vec2(x, -z) * bake.blend[k].w;
}

fn masked(k: u32) -> bool {
    return bake.blend[k].z > 0.5;
}

// The normal pass: the stacked tangent-space normal xy, stored as
// xy · ½ + ½ (the draw reads it back as × 2 − 1).
@fragment
fn bake_normal(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let c = covers(in.uv);
    var nxy = vec2<f32>(0.0);
    for (var k = 0u; k < bake.layers; k++) {
        if !covered(c, k) {
            continue;
        }
        let uv = layer_uv(in.uv, k);
        let nk = normal_xy(layer_normal(k, uv), bake.normal[k].z);
        if k == 0u {
            nxy = nk;
        } else {
            let m = layer_surface(k, uv).b;
            nxy = stack_xy(nxy, nk, c.a[k], m, bake.normal[k], masked(k));
        }
    }
    return vec4(nxy * 0.5 + 0.5, 0.0, 1.0);
}

// The colour pass: each layer's diffuse × its multiplier, stacked. The
// target holds the composite before the draw's `MatDif`, clamped to 8 bits.
@fragment
fn bake_colour(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let c = covers(in.uv);
    var colour = vec3<f32>(0.0);
    for (var k = 0u; k < bake.layers; k++) {
        if !covered(c, k) {
            continue;
        }
        let uv = layer_uv(in.uv, k);
        let ck = layer_diffuse(k, uv).rgb * bake.colour[k].rgb;
        if k == 0u {
            colour = ck;
        } else {
            let m = layer_surface(k, uv).b;
            colour = stack_colour(colour, ck, c.a[k], m, bake.blend[k], bake.colour[k].a);
        }
    }
    return vec4(clamp(colour, vec3(0.0), vec3(1.0)), 1.0);
}

// The gloss pass: each layer's gloss texel × its gloss factor, stacked like
// the colour.
@fragment
fn bake_gloss(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let c = covers(in.uv);
    var gloss = vec3<f32>(0.0);
    for (var k = 0u; k < bake.layers; k++) {
        if !covered(c, k) {
            continue;
        }
        let su = layer_surface(k, layer_uv(in.uv, k));
        let g = vec3<f32>(su.g * bake.surface[k].x);
        if k == 0u {
            gloss = g;
        } else {
            gloss = stack_colour(gloss, g, c.a[k], su.b, bake.blend[k], bake.colour[k].a);
        }
    }
    return vec4(clamp(gloss.x, 0.0, 1.0));
}

// The displacement pass, at half the set's side: each layer's parallax
// height, stacked like the normals, clamped to the 8-bit target.
@fragment
fn bake_height(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let c = covers(in.uv);
    var h = 0.0;
    for (var k = 0u; k < bake.layers; k++) {
        if !covered(c, k) {
            continue;
        }
        let su = layer_surface(k, layer_uv(in.uv, k));
        let lh = layer_height(su.r, bake.surface[k]);
        if k == 0u {
            h = lh;
        } else {
            h = mix(h, lh, sharpen(c.a[k], su.b, bake.normal[k].x, bake.normal[k].y, masked(k)));
        }
    }
    return vec4(clamp(h, 0.0, 1.0));
}
