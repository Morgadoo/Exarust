// The original's lighting, shared by its surface programs (`exa.wesl` for
// models, the terrain shader): GGX specular with Schlick Fresnel and no 1/π
// (the normalisation is in the light colours), point lights falling off as
// (1 − d/range)⁴ with the original's cube-shadow filter, the sun, and the
// voxel GI ambient. Rules: docs/RENDERING.md.

import bevy_pbr::render::{
    mesh_view_bindings as view_bindings,
    mesh_view_types,
    clustered_forward as clustering,
    shadows,
};

// The level ambient scales the GI volume ×15 (passed ×16).
const GI_SCALE: f32 = 15.0;
// The environment cube's strength in reflections indoors (`Ambient.w`), and
// the cube's level standing in when a material has none.
const REFLECTION_STRENGTH: f32 = 8.0;
const ENVIRONMENT_FALLBACK: f32 = 0.02;
// The level ambient is passed ×16, as the original scales its cube lookup.
const AMBIENT_SCALE: f32 = 16.0;
// The sun's illuminance per unit of the original's intensity.
const SUN_LUX: f32 = 160.0;
const PI: f32 = 3.14159265;

fn schlick(f0: vec3<f32>, c: f32) -> vec3<f32> {
    return f0 + (vec3(1.0) - f0) * exp2((-5.55473 * c - 6.98316) * c);
}

struct Surface {
    n: vec3<f32>,
    v: vec3<f32>,
    ndv: f32,
    // 1 − glossiness: r2 = rough⁴.
    rough: f32,
    f0: vec3<f32>,
    // Legacy materials use the original's older program: no Fresnel per
    // light, specular tinted by the specular colour at the end.
    legacy: bool,
}

// Diffuse and specular from lights.
struct Lit {
    diffuse: vec3<f32>,
    specular: vec3<f32>,
}

// One light of colour `colour` from direction `l`.
fn light(s: Surface, l: vec3<f32>, colour: vec3<f32>) -> Lit {
    var out: Lit;
    let ndl = max(dot(s.n, l), 0.0);
    if ndl <= 0.0 {
        return out;
    }
    let h = normalize(l + s.v);
    let ndh = clamp(dot(s.n, h), 0.0, 1.0);
    let hdv = max(dot(h, s.v), 0.0);
    let r2 = pow(s.rough, 4.0);
    let k = (s.rough + 1.0) * (s.rough + 1.0) / 8.0;
    let denom = ndh * ndh * (r2 - 1.0) + 1.0;
    // A mirror (roughness 0, as the terrain's full gloss gives) has no
    // highlight but at N·H = 1, where the formula is 0/0; the floor keeps
    // that point at 0 instead of NaN and never touches another value (with
    // roughness 0.02 the divisor is at least 3e-15).
    let dg = r2 * ndl / max(denom * denom * (ndl * (1.0 - k) + k), 1e-30);
    let d = 0.25 / (s.ndv * (1.0 - k) + k);
    if s.legacy {
        out.specular = dg * d * colour;
        out.diffuse = ndl * colour;
        return out;
    }
    let fs = schlick(s.f0, hdv);
    out.specular = dg * d * fs * colour;
    out.diffuse = ndl * (vec3(1.0) - fs) * colour;
    return out;
}

// Skin lit by one light, as the original's subsurface program lights it:
// plain N·L diffuse without the Fresnel share, GGX specular as `light`, and
// the light scattered under the skin (`scatter`), read against the blurred
// normal `sn`. The scatter wraps past the terminator: with b = sn·L and
// c = 0.5 + 0.05·blur, it is min(max(b + c, 0)²/2, ½) + max(b − 1 + c, 0),
// so it reaches 1.1 facing the light and fades out at b = −c.
struct LitSkin {
    diffuse: vec3<f32>,
    scatter: vec3<f32>,
    specular: vec3<f32>,
}

fn light_skin(s: Surface, sn: vec3<f32>, blur: f32, l: vec3<f32>, colour: vec3<f32>) -> LitSkin {
    var out: LitSkin;
    let b = dot(sn, l);
    let c = 0.5 + 0.05 * blur;
    let w = max(b + c, 0.0);
    out.scatter = (min(w * w * 0.5, 0.5) + max(b - 1.0 + c, 0.0)) * colour;
    let ndl = max(dot(s.n, l), 0.0);
    if ndl <= 0.0 {
        return out;
    }
    let lit = light(s, l, colour);
    out.specular = lit.specular;
    out.diffuse = ndl * colour;
    return out;
}

// The original's cube-shadow filter: taps of a fixed eight-point disc around
// the light direction, turned per window pixel, over the light's filter
// radius (`PointLight::radius`: (softness + 1)·0.005), each a hardware 2 × 2
// comparison; a normal offset of 0.008·(1 − |N·L|)·d and a 0.5 % depth bias.
// The GI lookup takes the same taps. The pairs are the first two lanes of
// the engine's `SdwSmp` table (three floats a tap), at their exact values.
const SHADOW_TAPS = array<vec2<f32>, 8>(
    vec2(0.27254149317741394, -0.5486504435539246),
    vec2(-0.28799688816070557, 0.6185606122016907),
    vec2(0.36168748140335083, 0.18059517443180084),
    vec2(-0.5052395462989807, -0.23951339721679688),
    vec2(-0.3027792274951935, -0.6664453148841858),
    vec2(-0.8587824106216431, 0.47176772356033325),
    vec2(0.3960069417953491, 0.7947491407394409),
    vec2(0.7731252908706665, -0.3697291910648346),
);
const SHADOW_DEPTH_SCALE: f32 = 0.995;
// A node's shadow-bias level v (0–7, the mesh tag's low bits) scales a
// receiver's distances by 1/k_r and a caster's by 1/k_c, with
// k_r = 1 − 0.005·v and k_c = k_r − 0.01 when v > 0 (terrain and floors 3,
// grid set pieces 1). Exarust applies only the receiver side, as the margin
// a surface keeps against itself: its distances are compared
// 0.995·k_c/k_r short (1.5 % instead of 0.5 %).
const SHADOW_BIAS_MASK: u32 = 7u;
const SHADOW_NORMAL_OFFSET: f32 = 0.008;
const FLIP_Z = vec3(1.0, 1.0, -1.0);

// The disc taps a fragment takes, and the disc's turn (sin, cos).
struct Taps {
    first: i32,
    count: i32,
    turn: vec2<f32>,
}

// All eight taps; or, in a supersampled frame (`split`), the two of the
// fragment's sub-frame, as the original spreads the taps over its four
// sub-frames: each 2 × 2 block of the scene image is one window pixel and
// its four pixels are the sub-frames, so the resolve averages all eight.
fn taps(frag: vec2<f32>, split: bool) -> Taps {
    var t: Taps;
    var pixel = frag;
    t.first = 0;
    t.count = 8;
    if split {
        let cell = vec2<i32>(floor(frag));
        t.first = 2 * ((cell.x & 1) + 2 * (cell.y & 1));
        t.count = 2;
        pixel = floor(frag * 0.5) + 0.5;
    }
    let turn = PI * fract(3.555713358 * pixel.x + 0.309269245 * pixel.y);
    t.turn = vec2(sin(turn), cos(turn));
    return t;
}

// Disc tap `j` turned by `t`.
fn tap(t: Taps, j: i32) -> vec2<f32> {
    let k = SHADOW_TAPS[j];
    return vec2(k.x * t.turn.y - k.y * t.turn.x, k.x * t.turn.x + k.y * t.turn.y);
}

// The receiver's distance scale for shadow-bias level `v`.
fn shadow_depth_scale(v: u32) -> f32 {
    if v == 0u {
        return SHADOW_DEPTH_SCALE;
    }
    let kr = 1.0 - 0.005 * f32(v);
    return SHADOW_DEPTH_SCALE * (kr - 0.01) / kr;
}

// The shadow-bias level carried in a mesh tag.
fn shadow_bias_level(tag: u32) -> u32 {
    return tag & SHADOW_BIAS_MASK;
}

// Two axes across direction `d` (unit), as the original builds its disc
// frame.
fn disc_frame(d: vec3<f32>) -> array<vec3<f32>, 2> {
    let ia = 1.0 / (1.00001 + d.z);
    let bb = -d.x * d.y * ia;
    return array<vec3<f32>, 2>(vec3(1.0 - d.x * d.x * ia, bb, -d.x), vec3(bb, 1.0 - d.y * d.y * ia, -d.y));
}

fn point_shadow(id: u32, p: vec3<f32>, nrm: vec3<f32>, t: Taps, depth_scale: f32) -> f32 {
    let lt = &view_bindings::clustered_lights.data[id];
    let l = (*lt).position_radius.xyz - p;
    let d = max(length(l), 1e-4);
    let ln = l / d;
    // Toward the light from the surface pushed out along the normal.
    let push = (1.0 - abs(dot(nrm, ln))) * d * SHADOW_NORMAL_OFFSET;
    let s_vec = (l - nrm * push) / d;
    let axes = disc_frame(ln);
    let radius = (*lt).position_radius.w;
    var lit = 0.0;
    for (var j = t.first; j < t.first + t.count; j++) {
        let rs = tap(t, j);
        let dir = (axes[0] * rs.x + axes[1] * rs.y) * radius - s_vec;
        // The surface's distance along this tap, in Bevy's cube depth.
        let q = normalize(dir) * d * depth_scale;
        let major = max(abs(q.x), max(abs(q.y), abs(q.z)));
        let zw = -major * (*lt).light_custom_data.xy + (*lt).light_custom_data.zw;
        lit += cube_compare(dir * FLIP_Z, id, zw.x / zw.y);
    }
    return lit / f32(t.count);
}

// Exarust's cached cube shadow maps of static casters
// (`exa_render::cube_shadows`): the original's linear depth, distance /
// (R·k_c), compared against 0.995·d / (R·k_r) with the receiver's k_r.
@group(constants::MATERIAL_BIND_GROUP) @binding(131) var exa_cubes: texture_depth_cube_array;
@group(constants::MATERIAL_BIND_GROUP) @binding(132) var exa_cube_sampler: sampler_comparison;

// The receiver depth scale k_r of shadow-bias level `v`.
fn receiver_scale(v: u32) -> f32 {
    if v == 0u {
        return 1.0;
    }
    return 1.0 - 0.005 * f32(v);
}

// The cached cube's filter, with the same taps as the point-shadow filter.
fn cached_shadow(slot: u32, reach: f32, id: u32, p: vec3<f32>, nrm: vec3<f32>, t: Taps, level: u32) -> f32 {
    let lt = &view_bindings::clustered_lights.data[id];
    let l = (*lt).position_radius.xyz - p;
    let d = max(length(l), 1e-4);
    let ln = l / d;
    let push = (1.0 - abs(dot(nrm, ln))) * d * SHADOW_NORMAL_OFFSET;
    let s_vec = (l - nrm * push) / d;
    let axes = disc_frame(ln);
    let radius = (*lt).position_radius.w;
    let reference = SHADOW_DEPTH_SCALE * d / (reach * receiver_scale(level));
    var lit = 0.0;
    for (var j = t.first; j < t.first + t.count; j++) {
        let rs = tap(t, j);
        let dir = (axes[0] * rs.x + axes[1] * rs.y) * radius - s_vec;
        lit += textureSampleCompareLevel(exa_cubes, exa_cube_sampler, dir * FLIP_Z, i32(slot), reference);
    }
    return lit / f32(t.count);
}

// The shadow of clustered light `id` on a receiver with mesh tag `tag`: the
// cached cube of its static casters times Bevy's cube of its skinned ones.
fn light_shadow(id: u32, p: vec3<f32>, nrm: vec3<f32>, t: Taps, tag: u32, receiver: bool) -> f32 {
    if !receiver {
        return 1.0;
    }
    var shadow = 1.0;
    let rec = exa_lights[1u + light_id(id)];
    if rec.z > 0u {
        shadow = cached_shadow(rec.z - 1u, bitcast<f32>(rec.w), id, p, nrm, t, shadow_bias_level(tag));
    }
    let lt = &view_bindings::clustered_lights.data[id];
    if ((*lt).flags & mesh_view_types::POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u {
        shadow *= point_shadow(id, p, nrm, t, shadow_depth_scale(shadow_bias_level(tag)));
    }
    return shadow;
}

fn cube_compare(dir: vec3<f32>, id: u32, depth: f32) -> f32 {
    @if(NO_CUBE_ARRAY_TEXTURES_SUPPORT)
    return textureSampleCompare(
        view_bindings::point_shadow_textures,
        view_bindings::point_shadow_textures_comparison_sampler,
        dir,
        depth,
    );
    @else
    return textureSampleCompareLevel(
        view_bindings::point_shadow_textures,
        view_bindings::point_shadow_textures_comparison_sampler,
        dir,
        i32(id),
        depth,
    );
}

// Light records and mesh light lists (`exa_render::lights`), bound to every
// surface material: element 0 holds the first list's index, element 1 + id
// light `id`'s shaping (y, z), each list two elements (count, then ids).
@group(constants::MATERIAL_BIND_GROUP) @binding(130) var<storage, read> exa_lights: array<vec4<u32>>;

// A clustered light's id in the records, carried above Bevy's default point
// bias. The field itself must remain a valid shadow bias: skinned characters
// still cast through Bevy's shadow maps.
const LIGHT_ID_BIAS: f32 = 0.08;
const LIGHT_ID_STEP: f32 = 0.00001;

fn light_id(index: u32) -> u32 {
    let bias = view_bindings::clustered_lights.data[index].shadow_depth_bias;
    return u32(round((bias - LIGHT_ID_BIAS) / LIGHT_ID_STEP));
}

// Whether the mesh with tag `tag` is lit by clustered light `index`: a mesh
// with a light list (tag bits 3 and up) only by the lights on it, as the
// original lights an object by its strongest links; others by every light.
fn lit_by(tag: u32, index: u32) -> bool {
    let slot = tag >> 3u;
    if slot == 0u {
        return true;
    }
    let base = exa_lights[0].x + 2u * (slot - 1u);
    let a = exa_lights[base];
    let b = exa_lights[base + 1u];
    let id = light_id(index);
    let n = a.x;
    return (n > 0u && a.y == id) || (n > 1u && a.z == id) || (n > 2u && a.w == id)
        || (n > 3u && b.x == id) || (n > 4u && b.y == id) || (n > 5u && b.z == id);
}

// A level light's vertical shaping for the vertical part `ly` of the
// direction to it (1 straight below): max((2^(z·(ly + (1 − ly)·y)) − 1) /
// (2^z − 1), 0), with y and z from the light's record. Unshaped lights
// carry (1, 1): a factor of 1.
fn vertical_shaping(index: u32, ly: f32) -> f32 {
    let rec = exa_lights[1u + light_id(index)];
    let y = bitcast<f32>(rec.x);
    let z = bitcast<f32>(rec.y);
    let a = (ly + (1.0 - ly) * y) * z;
    return max((exp2(a) - 1.0) / max(exp2(z) - 1.0, 1e-4), 0.0);
}

// The fragment's view depth, for the light clusters and the sun's cascades.
fn view_depth(world_position: vec4<f32>) -> f32 {
    return dot(vec4<f32>(
        view_bindings::view.view_from_world[0].z,
        view_bindings::view.view_from_world[1].z,
        view_bindings::view.view_from_world[2].z,
        view_bindings::view.view_from_world[3].z,
    ), world_position);
}

// Every point light and the sun on surface `s` at `world_position`, with
// shadows when the mesh receives them (`nrm` offsets the shadow lookups,
// `t` are the fragment's disc taps, `tag` its mesh tag: shadow-bias level and
// light list).
fn direct(s: Surface, world_position: vec4<f32>, nrm: vec3<f32>, frag: vec2<f32>, t: Taps, receiver: bool, tag: u32) -> Lit {
    var out: Lit;
    let p = world_position.xyz;
    let view_z = view_depth(world_position);
    let ortho = view_bindings::view.clip_from_view[3].w == 1.0;
    let cluster = clustering::view_fragment_cluster_index(frag, view_z, ortho);
    let ranges = clustering::unpack_clusterable_object_index_ranges(cluster);
    for (var i = ranges.first_point_light_index_offset; i < ranges.first_spot_light_index_offset; i++) {
        let id = clustering::get_clusterable_object_id(i);
        if !lit_by(tag, id) {
            continue;
        }
        let lt = &view_bindings::clustered_lights.data[id];
        let to = (*lt).position_radius.xyz - p;
        let d = length(to);
        let atn = pow(max(1.0 - d / max((*lt).range, 1e-3), 0.0), 4.0) * vertical_shaping(id, to.y / max(d, 1e-4));
        if atn <= 0.0 {
            continue;
        }
        let shadow = light_shadow(id, p, nrm, t, tag, receiver);
        let lit = light(s, to / max(d, 1e-4), (*lt).color_inverse_square_range.rgb * atn * shadow);
        out.diffuse += lit.diffuse;
        out.specular += lit.specular;
    }
    for (var i = 0u; i < view_bindings::lights.n_directional_lights; i++) {
        let sun = &view_bindings::lights.directional_lights[i];
        var shadow = 1.0;
        if receiver && ((*sun).flags & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u {
            shadow = shadows::fetch_directional_shadow(i, world_position, nrm, view_z, frag);
        }
        let lit = light(s, (*sun).direction_to_light, (*sun).color.rgb / SUN_LUX * shadow);
        out.diffuse += lit.diffuse;
        out.specular += lit.specular;
    }
    return out;
}

// The GI volume around a surface point: the fragment's disc taps (`t`),
// spread half a unit across the normal and taken 20 cm out along it; light
// and openness averaged. `origin` and `extent` place the volume (metres).
fn gi_sample(
    volume: texture_3d<f32>,
    volume_sampler: sampler,
    origin: vec3<f32>,
    extent: vec3<f32>,
    p: vec3<f32>,
    n: vec3<f32>,
    t: Taps,
) -> vec4<f32> {
    // The original's frame across the normal, with its 0.1 clamp that keeps
    // the 1/(1 + n.z) term finite toward −Z.
    let a = 1.0 / max(1.0 + n.z, 0.1);
    let cross_xy = -n.x * n.y * a;
    let ax = vec3(1.0 - n.x * n.x * a, cross_xy, -n.x);
    let ay = vec3(cross_xy, 1.0 - n.y * n.y * a, -n.y);
    var sum = vec4(0.0);
    for (var j = t.first; j < t.first + t.count; j++) {
        let rs = tap(t, j);
        let d = n + (ax * rs.x + ay * rs.y) * 0.5;
        let w = (p + d * 0.2 - origin) / extent;
        sum += textureSampleLevel(volume, volume_sampler, w.xzy, 0.0);
    }
    return sum / f32(t.count);
}

// Whether `p` lies inside the GI volume.
fn in_gi(origin: vec3<f32>, extent: vec3<f32>, p: vec3<f32>) -> bool {
    let w = (p - origin) / extent;
    return all(w >= vec3(0.0)) && all(w <= vec3(1.0));
}

// The ambient from a GI sample `g` (light, openness): lifted where faint,
// × the level ambient (`level`, passed ×16) × 15 × openness.
fn gi_ambient(g: vec4<f32>, level: vec3<f32>) -> vec3<f32> {
    let a = length(g.rgb);
    let b = (1.0 - a) * (1.0 - a) * 0.03 / (a + 0.016) + 0.47 * a + 0.53;
    return g.rgb * level / AMBIENT_SCALE * GI_SCALE * b * g.a;
}
