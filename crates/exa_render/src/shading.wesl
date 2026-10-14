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
    let ndh = max(dot(s.n, h), 0.0);
    let hdv = max(dot(h, s.v), 0.0);
    let r2 = pow(s.rough, 4.0);
    let k = (s.rough + 1.0) * (s.rough + 1.0) / 8.0;
    let denom = ndh * ndh * (r2 - 1.0) + 1.0;
    let dg = r2 * ndl / (denom * denom * (ndl * (1.0 - k) + k));
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

// The original's cube-shadow filter: eight taps of a fixed disc around the
// light direction, turned per pixel, over the light's filter radius
// (`PointLight::radius`: (softness + 1)·0.005), each a hardware 2 × 2
// comparison; a normal offset of 0.008·(1 − |N·L|)·d and a 0.5 % depth bias.
const SHADOW_TAPS = array<vec2<f32>, 8>(
    vec2(0.2725, -0.5487), vec2(-0.2880, 0.6186), vec2(0.3617, 0.1806), vec2(-0.5052, -0.2395),
    vec2(-0.3028, -0.6664), vec2(-0.8588, 0.4718), vec2(0.3960, 0.7947), vec2(0.7731, -0.3697),
);
const SHADOW_DEPTH_SCALE: f32 = 0.995;
const SHADOW_NORMAL_OFFSET: f32 = 0.008;
const FLIP_Z = vec3(1.0, 1.0, -1.0);

fn point_shadow(id: u32, p: vec3<f32>, nrm: vec3<f32>, frag: vec2<f32>) -> f32 {
    let lt = &view_bindings::clustered_lights.data[id];
    let l = (*lt).position_radius.xyz - p;
    let d = max(length(l), 1e-4);
    let ln = l / d;
    // Toward the light from the surface pushed out along the normal.
    let push = (1.0 - abs(dot(nrm, ln))) * d * SHADOW_NORMAL_OFFSET;
    let s_vec = (l - nrm * push) / d;
    // A frame around the light direction.
    let ia = 1.0 / (1.00001 + ln.z);
    let bb = -ln.x * ln.y * ia;
    let ax = vec3(1.0 - ln.x * ln.x * ia, bb, -ln.x);
    let ay = vec3(bb, 1.0 - ln.y * ln.y * ia, -ln.y);
    let turn = PI * fract(3.555713358 * frag.x + 0.309269245 * frag.y);
    let rb = vec2(sin(turn), cos(turn));
    let radius = (*lt).position_radius.w;
    var lit = 0.0;
    for (var j = 0; j < 8; j++) {
        let t = SHADOW_TAPS[j];
        let rs = vec2(t.x * rb.y - t.y * rb.x, t.x * rb.x + t.y * rb.y);
        let dir = (ax * rs.x + ay * rs.y) * radius - s_vec;
        // The surface's distance along this tap, in Bevy's cube depth.
        let q = normalize(dir) * d * SHADOW_DEPTH_SCALE;
        let major = max(abs(q.x), max(abs(q.y), abs(q.z)));
        let zw = -major * (*lt).light_custom_data.xy + (*lt).light_custom_data.zw;
        lit += cube_compare(dir * FLIP_Z, id, zw.x / zw.y);
    }
    return lit / 8.0;
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

// A level light's vertical shaping for the vertical part `ly` of the
// direction to it (1 straight below): max((2^(z·(ly + (1 − ly)·y)) − 1) /
// (2^z − 1), 0), with y and z carried in the light's shadow bias fields.
// Unshaped lights carry (1, 1): a factor of 1.
fn vertical_shaping(id: u32, ly: f32) -> f32 {
    let lt = &view_bindings::clustered_lights.data[id];
    let y = (*lt).shadow_depth_bias;
    let z = (*lt).shadow_normal_bias;
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
// shadows when the mesh receives them (`nrm` offsets the shadow lookups).
fn direct(s: Surface, world_position: vec4<f32>, nrm: vec3<f32>, frag: vec2<f32>, receiver: bool) -> Lit {
    var out: Lit;
    let p = world_position.xyz;
    let view_z = view_depth(world_position);
    let ortho = view_bindings::view.clip_from_view[3].w == 1.0;
    let cluster = clustering::view_fragment_cluster_index(frag, view_z, ortho);
    let ranges = clustering::unpack_clusterable_object_index_ranges(cluster);
    for (var i = ranges.first_point_light_index_offset; i < ranges.first_spot_light_index_offset; i++) {
        let id = clustering::get_clusterable_object_id(i);
        let lt = &view_bindings::clustered_lights.data[id];
        let to = (*lt).position_radius.xyz - p;
        let d = length(to);
        let atn = pow(max(1.0 - d / max((*lt).range, 1e-3), 0.0), 4.0) * vertical_shaping(id, to.y / max(d, 1e-4));
        if atn <= 0.0 {
            continue;
        }
        var shadow = 1.0;
        if receiver && ((*lt).flags & mesh_view_types::POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u {
            shadow = point_shadow(id, p, nrm, frag);
        }
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

// The GI volume around a surface point: four taps 20 cm out along the
// normal, spread across the tangent plane; light and openness averaged.
// `origin` and `extent` place the volume (metres).
fn gi_sample(
    volume: texture_3d<f32>,
    volume_sampler: sampler,
    origin: vec3<f32>,
    extent: vec3<f32>,
    p: vec3<f32>,
    n: vec3<f32>,
) -> vec4<f32> {
    let a = select(vec3(0.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0), abs(n.y) > 0.9);
    let ax = normalize(cross(n, a));
    let ay = cross(n, ax);
    var sum = vec4(0.0);
    for (var i = 0; i < 4; i++) {
        let sx = select(-0.5, 0.5, (i & 1) == 1);
        let sy = select(-0.5, 0.5, (i & 2) == 2);
        let d = n + ax * sx + ay * sy;
        let w = (p + d * 0.2 - origin) / extent;
        sum += textureSampleLevel(volume, volume_sampler, w.xzy, 0.0);
    }
    return sum * 0.25;
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
