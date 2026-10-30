// One step of the original's voxel global illumination, a light-propagation
// volume: each open voxel gathers its eight diagonal neighbours' light
// (weighted by how open they are), keeps most of it, gains a faint base
// ambient and the direct light of its sector's lights; point lights are
// shadowed by their cube maps and the sky light, which shines straight
// down, by what stands above the voxel in its column. Voxel
// lights (glowing items) then add their light straight into the voxels
// around them. Light builds up over many steps. Storage is 8-bit sRGB like
// the original's.
//
// The original rasterises one quad per slice with a layered framebuffer and
// reads its 3D light texture at the layer's slice centre (the vertex
// shader's -(layer + 0.5)/D), so the ±1.33-voxel taps blend the neighbouring
// slices; this compute pass writes the same voxel of each slice and samples
// at the same coordinates.
// Rules: docs/RENDERING.md (Voxel GI).

struct GiLight {
    // A point light's position in texture units (x, z depth, y height) and
    // the inverse of its radius in those units. Unused for the sky light.
    position_radius: vec4<f32>,
    // Per-step injection colour: linear colour × intensity × 0.15 (×0.9 for
    // the sky light), before the fragment's 0.01; in w the cube lookup's depth
    // scale, ((bias + 0.94)/R)·5120 with the light's shadow bias (0.025 at
    // the shipped detail), so the compare value is d·(bias + 0.94)/R + 0.01.
    colour_shadow: vec4<f32>,
    // x, y: the light's vertical shaping (y, z); z: 1/(2^z − 1) (unshaped
    // lights carry (1, 1, 1)); w: 0 for a point light, 1 for the sky light, the
    // original's two program branches.
    shaping: vec4<f32>,
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

// A registered emissive shape: the manager's clipped draw box, its program
// and the uniforms its fragment body reads. Shape-space lengths are the
// original's centimetres.
struct GiShape {
    // x: the program, y: the first slice, z: the slice count.
    head: vec4<u32>,
    // The clipped column box: lo X, lo Z, hi X, hi Z.
    clip: vec4<u32>,
    colour: vec4<f32>,
    // x: Atten, y: Radius, z: Fade, w: Pulse.
    params: vec4<f32>,
    // x: Anim.
    extra: vec4<f32>,
    // bMat: world centimetres into the shape's space.
    matrix: mat4x4<f32>,
    half: vec4<f32>,
    tex_pos: vec4<f32>,
    line0: vec4<f32>,
    line1: vec4<f32>,
    grad: vec4<f32>,
    point_light: vec4<f32>,
    point_pos: array<vec4<f32>, 8>,
    point_col: array<vec4<f32>, 8>,
}

struct GiParams {
    // Lowest corner of the block (m).
    origin: vec4<f32>,
    // Size of the block (m).
    extent: vec4<f32>,
    // The base ambient every open voxel gains per step, per channel.
    base: vec4<f32>,
    // z: 1 when the steps reach the top slice (the level record's flag).
    flags: vec4<u32>,
    groups: array<GiGroup, 64>,
    // Per sector (X fastest), its group or 0xffffffff.
    sector_groups: array<vec4<u32>, 16>,
    lights: array<GiLight, 768>,
    // Each light's cached cube slot (0xffffffff for none): the original
    // binds a light's cube map to its own `Shadow<i>` sampler.
    cubes: array<vec4<u32>, 192>,
    // x = the number of voxel lights.
    voxel_count: vec4<u32>,
    voxel_lights: array<GiVoxelLight, 64>,
    // x = the number of registered shapes.
    shape_count: vec4<u32>,
    shapes: array<GiShape, 16>,
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
// Per column (x, z depth), the lowest slice the sky light reaches.
@group(0) @binding(7) var sky_floor: texture_2d<u32>;
// The shape programs' texture (`Tex0`); the manager's shapes share one.
@group(0) @binding(8) var shape_texture: texture_2d<f32>;

fn to_srgb(c: vec3<f32>) -> vec3<f32> {
    let l = clamp(c, vec3(0.0), vec3(1.0));
    return select(1.055 * pow(l, vec3(1.0 / 2.4)) - 0.055, l * 12.92, l <= vec3(0.0031308));
}

fn openness(occ: f32) -> f32 {
    return max(1.0 - 13.0 * occ, 0.0);
}

// The fragment the original's layered quad writes and the coordinates it
// reads: the layer's slice centre, as the vertex shader's
// `-(layer + 0.5)/D` gives it, and x and y at the texel centres. So each
// voxel reads its own slice exactly; only the ±1.33-voxel taps blend the
// neighbouring slices.
fn volume_uv(id: vec3<u32>, dims: vec3<u32>) -> vec3<f32> {
    return vec3(
        (f32(id.x) + 0.5) / f32(dims.x),
        (f32(id.y) + 0.5) / f32(dims.y),
        (f32(id.z) + 0.5) / f32(dims.z),
    );
}

// The original's world-scale volume coordinates: x and y as texture units,
// z the height divided by the block's width (not its slice count), so one
// vertical voxel is one 1/width step and the slice's middle sits at
// (slice + 0.5)/width.
fn volume_height(id: vec3<u32>, dims: vec3<u32>) -> vec3<f32> {
    return vec3(
        (f32(id.x) + 0.5) / f32(dims.x),
        (f32(id.y) + 0.5) / f32(dims.y),
        (f32(id.z) + 0.5) / f32(dims.x),
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

// The registered shapes' contribution to voxel `id` this step: the
// original's per-shape draw over its clipped box, evaluated per voxel. The
// six bodies are its `shading.flb` fragment programs (see
// `exa_render::gi_shape`, which evaluates the same bodies on the CPU).
fn gl_min(x: f32, y: f32) -> f32 { return select(x, y, y < x); }
fn gl_max(x: f32, y: f32) -> f32 { return select(x, y, x < y); }
fn gl_clamp(x: f32, lo: f32, hi: f32) -> f32 { return gl_min(gl_max(x, lo), hi); }
fn gl_mix(x: f32, y: f32, a: f32) -> f32 { return x * (1.0 - a) + y * a; }
fn gl_sign(x: f32) -> f32 {
    if x > 0.0 { return 1.0; }
    if x < 0.0 { return -1.0; }
    return x * 0.0;
}
// The shipped drivers' `round`: a half away from zero, as `gi_shape`'s CPU
// body takes it [guess, as there]; WGSL's own `round` takes a half to even.
fn gl_round(x: f32) -> f32 {
    let t = trunc(x);
    if abs(x - t) >= 0.5 { return t + sign(x); }
    return t;
}
fn gl_max3(a: vec3<f32>, b: vec3<f32>) -> vec3<f32> {
    return vec3(gl_max(a.x, b.x), gl_max(a.y, b.y), gl_max(a.z, b.z));
}

fn shape_texel(uv: vec2<f32>) -> f32 {
    return textureSampleLevel(shape_texture, linear_sampler, uv, 0.0).r;
}

// `gi_dome`: a shell at radius 100 with the dome texture's ridge cubed, a
// 70-unit inner cap and a 1 − d²/1000 shell falloff, times 6.
fn shape_dome(s: GiShape, p: vec3<f32>) -> vec3<f32> {
    let tc = p.xy * 0.75 * 0.00375 + s.tex_pos.xy;
    let a0 = shape_texel(tc);
    let a = 1.0 - abs(a0 - 0.4) * 2.0;
    var l = abs(100.0 - length(p));
    l = l * l;
    let v = p - vec3(0.0, 0.0, 100.0);
    var b = gl_max(length(v) - 70.0, 0.0);
    b = gl_max(1.0 - b * 0.02, 0.0);
    var atn = gl_max(1.0 - l * 0.001, 0.0);
    atn = atn * b;
    atn = atn * 6.0 * a * a * a;
    return gl_max3(s.colour.rgb * atn, vec3(0.0));
}

// `gi_cuboid`: the distance outside the box, falling off as 1 − d²·0.0002.
fn shape_cuboid(s: GiShape, p: vec3<f32>) -> vec3<f32> {
    let d = gl_max3(abs(p) - s.half.xyz, vec3(0.0));
    let l = dot(d, d);
    let atn = gl_max(1.0 - l * 0.0002, 0.0);
    return gl_max3(s.colour.rgb * atn, vec3(0.0));
}

// `gi_bubble`: a shell at Radius with the bubble texture cubed, the vertical
// Fade cut and 4× the shell falloff.
fn shape_bubble(s: GiShape, p: vec3<f32>) -> vec3<f32> {
    let tc = vec2(p.x, p.z) * 0.75 * 0.00375 + s.tex_pos.xy;
    let a0 = shape_texel(tc);
    var a = 1.0 - abs(a0 - 0.4) * 2.0;
    a = a * a * a;
    let b = p.y / length(p);
    let f = (1.5 + b) * s.params.z;
    a = a * gl_clamp(8.0 - f * 16.0, 0.0, 1.0);
    var l = abs(s.params.y - length(p));
    l = l * l;
    let atn = gl_max(1.0 - l * 0.001, 0.0);
    return gl_max3(s.colour.rgb * (atn * 4.0 * a), vec3(0.0));
}

// `gi_telebubble`: the teleporter's pulsing bubble.
fn shape_telebubble(s: GiShape, p: vec3<f32>) -> vec3<f32> {
    let l_dst = length(p);
    let nrm = p / l_dst;
    var f = nrm.y * 0.5 + 0.5;
    let a = (1.0 - f * 0.5) * 0.007;
    var tcz = vec2(p.x, p.y) * a;
    var tcy = vec2(p.x, p.z) * a;
    var tcx = vec2(p.y, p.z) * a;
    tcx.x += gl_sign(nrm.x) * 0.333;
    tcy.y += gl_sign(nrm.y) * 0.333;
    tcz.x -= gl_sign(nrm.z) * 0.333;
    let tbw = abs(nrm * nrm) / (nrm.x * nrm.x + nrm.y * nrm.y + nrm.z * nrm.z);
    var i1 = shape_texel(tcz * 0.5 + s.tex_pos.xy) * tbw.z;
    i1 += shape_texel(tcx * 0.5 + s.tex_pos.xy) * tbw.x;
    i1 += shape_texel(tcy * 0.5 + s.tex_pos.xy) * tbw.y;
    let i1q = i1 * i1 * 2.0;
    let fa = 1.0 - abs(f * 2.0 - 1.0);
    var aa = gl_mix(fa, 1.0 - i1q, 0.4);
    f = gl_max(s.params.z, 0.0);
    aa = pow(aa, 16.0 * f);
    let c = aa;
    // The body squares `1 − |0.5 − c|·2` and overwrites it with the pulse
    // term before reading it, as the original does.
    var sc = i1q * gl_round(c);
    sc *= 1.0 + abs(s.params.z);
    var an = abs(nrm.y) - s.extra.x;
    an -= gl_round(an);
    let ap = an * an * s.params.w * 4.0;
    sc += ap * i1q;
    f = gl_max(f - 0.25, 0.0);
    f = 1.0 + f * 4.0;
    sc = (sc + ap * 0.5) * f;
    var l = abs(s.params.y - l_dst);
    l = l * l;
    let atn = gl_max(1.0 - l * 0.001, 0.0);
    let out = s.colour.rgb * sc * 4.0;
    return gl_max3(out * out * atn * 4.0, vec3(0.0));
}

// `gi_gradline`: a lit segment with a two-ended gradient and one extra point
// light.
fn shape_gradline(s: GiShape, p: vec3<f32>) -> vec3<f32> {
    let lv = s.line1.xyz - s.line0.xyz;
    let sv = p - s.line0.xyz;
    let l = gl_clamp(dot(lv, sv) / dot(lv, lv), 0.0, 1.0);
    let near = s.line0.xyz + lv * l;
    let a = gl_max(s.grad.x - l, 0.0);
    let b = gl_max(l - s.grad.x, 0.0);
    let w = a * s.grad.y + b * s.grad.z + (1.0 - a - b);
    let dir = p - near;
    let d = dot(dir, dir);
    var line_atn = gl_max(1.0 - d * s.params.x, 0.0);
    line_atn = line_atn * line_atn;
    var out = s.colour.rgb * line_atn * w * 3.0;
    let d2 = p - s.point_light.xyz;
    let dd = dot(d2, d2);
    var atn2 = gl_max(1.0 - dd * s.params.x, 0.0);
    atn2 = atn2 * atn2;
    out += s.colour.rgb * atn2 * s.point_light.w * w;
    return gl_max3(out, vec3(0.0));
}

// `gi_multipoint`: each point adds its colour times max(1 − d²·Atten, 0)²·0.25.
// The occlusion term the body computes is never applied.
fn shape_multipoint(s: GiShape, p: vec3<f32>) -> vec3<f32> {
    var out = vec3(0.0);
    for (var i = 0u; i < 8u; i++) {
        let dir = p - s.point_pos[i].xyz;
        let d = dot(dir, dir);
        let atn = gl_max(1.0 - d * s.params.x, 0.0);
        out += s.point_col[i].rgb * (atn * atn) * 0.25;
    }
    return gl_max3(out, vec3(0.0));
}

// The shapes covering voxel `id` (texture axes X, Z, Y), with `world` the
// voxel's centre in world centimetres and `occ` its occupancy.
fn shape_light(id: vec3<u32>, world: vec3<f32>, occ: f32) -> vec3<f32> {
    var rgb = vec3(0.0);
    for (var i = 0u; i < params.shape_count.x; i++) {
        let s = params.shapes[i];
        if any(id.xy < s.clip.xy) || any(id.xy >= s.clip.zw) {
            continue;
        }
        let first = i32(s.head.y);
        if i32(id.z) < first || i32(id.z) >= first + i32(s.head.z) {
            continue;
        }
        let p = (s.matrix * vec4(world, 1.0)).xyz;
        let prog = s.head.x;
        if prog == 0u {
            rgb += shape_dome(s, p);
        } else if prog == 1u {
            rgb += shape_cuboid(s, p);
        } else if prog == 2u {
            rgb += shape_bubble(s, p);
        } else if prog == 3u {
            rgb += shape_telebubble(s, p);
        } else if prog == 4u {
            rgb += shape_gradline(s, world);
        } else {
            rgb += shape_multipoint(s, world);
        }
    }
    _ = occ;
    return rgb;
}

// The voxels the manager does not draw this frame: the original clears the
// scratch target to (0, 0, 0, 0) and then runs the voxel lights' additive
// pass over it, so they take the lights' colour and no openness.
fn write_dark(id: vec3<u32>, dims: vec3<u32>) {
    textureStore(light_next, vec3<i32>(id), vec4(to_srgb(voxel_lights(id, dims)), 0.0));
}

// The sky light's visibility at voxel `id`: the original's map looking
// straight down, taken by its first comparison alone under a sky, is lit
// where nothing stands above; Exarust reads the column's highest occluder.
// The voxel's openness then scales it.
fn sky_light(id: vec3<u32>, occ: f32) -> f32 {
    let floor = textureLoad(sky_floor, vec2<i32>(id.xy), 0).r;
    return select(0.0, occ, id.z >= floor);
}

// A neighbour block's margin copy (the original's `gi_blit`): the block
// being injected takes one margin column or row from a resident neighbour's
// light texture, at the same world position (the destination's own texture
// coordinates plus `offset`), every slice but the top one.
struct GiBlit {
    // The block's texels along X, Z and Y, and the slices to copy.
    dims: vec4<u32>,
    // The destination strip: lo X, lo Z, hi X, hi Z.
    strip: vec4<u32>,
    // Added to the destination's texture coordinates to read the neighbour.
    offset: vec4<f32>,
}

@group(0) @binding(0) var blit_source: texture_3d<f32>;
@group(0) @binding(1) var blit_sampler: sampler;
@group(0) @binding(2) var blit_target: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(3) var<uniform> blit: GiBlit;

@compute @workgroup_size(4, 4, 4)
fn margin_blit(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = blit.strip.z - blit.strip.x;
    let height = blit.strip.w - blit.strip.y;
    if id.x >= width || id.y >= height || id.z >= blit.dims.w {
        return;
    }
    let dest = vec3<u32>(blit.strip.x + id.x, blit.strip.y + id.y, id.z);
    let uv = (vec2<f32>(dest.xy) + 0.5) / vec2<f32>(blit.dims.xy) + blit.offset.xy;
    let r = (f32(id.z) + 0.5) / f32(blit.dims.z);
    let src = textureSampleLevel(blit_source, blit_sampler, vec3(uv, r), 0.0);
    // The neighbour is sampled through its sRGB view; the target stores
    // sRGB-encoded bytes, as the propagation does.
    textureStore(blit_target, vec3<i32>(dest), vec4(to_srgb(src.rgb), src.a));
}

@compute @workgroup_size(4, 4, 4)
fn propagate(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(light_next);
    if any(id >= dims) {
        return;
    }
    // The 8-voxel margins, and the top slice unless the level record lets
    // the groups draw it, are cleared every step; slices outside a sector
    // group's span and unmarked sectors are too.
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
    if id.z < plan.floor || (id.z + 1u >= dims.z && params.flags.z == 0u) {
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
    // its first light is the directional sky light: outdoors a neighbour's light counts by its
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
            // The sky light: no fall-off, shaded from above (the original's
            // directional `SdwMat` path).
            rgb += lt.colour_shadow.rgb * sky_light(id, open) * 0.01;
            continue;
        }
        let to = at - lt.position_radius.xyz;
        let d = length(to);
        var shadow = 1.0;
        // The original binds light i's cube map to `Shadow<i>`; the depth
        // compared is d·(bias + 0.94)/R, formed by the CPU's scale.
        let slot = params.cubes[(plan.first + i) / 4u][(plan.first + i) % 4u];
        if slot != 0xffffffffu {
            let dir = normalize(vec3(to.x, to.z, to.y)) * FLIP_Z;
            shadow = textureSampleCompareLevel(
                shadow_cubes,
                shadow_cube_sampler,
                dir,
                i32(slot),
                d * lt.colour_shadow.w + 0.01,
            );
        }
        var atn = max(1.0 - d * lt.position_radius.w, 0.0);
        atn = atn * atn;
        atn = atn * atn * open * shadow;
        let ly = -to.z / d;
        let shaped = max((exp2((ly + (1.0 - ly) * lt.shaping.x) * lt.shaping.y) - 1.0) * lt.shaping.z, 0.0);
        rgb += lt.colour_shadow.rgb * atn * shaped * 0.01;
    }
    // The shapes inject after the point lights and before the voxel lights,
    // as the original draws them.
    let world = params.origin.xyz
        + vec3(at.x * params.extent.x, at.z * params.extent.x, at.y * params.extent.z);
    rgb += shape_light(id, world * 100.0, occ0);
    rgb += voxel_lights(id, dims);
    textureStore(light_next, vec3<i32>(id), vec4(to_srgb(max(rgb, vec3(0.0))), stored_alpha));
}
