// The original's surface shading (its `shadeopt` program), on Bevy's lights.
// Linear workflow; GGX specular with Schlick Fresnel and no 1/π (the
// normalisation is in the light colours); point lights fall off as
// (1 − d/range)⁴; ambient comes from the level. Rules: docs/RENDERING.md.

import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    mesh_bindings::mesh,
    mesh_view_bindings as view_bindings,
    mesh_view_types,
    mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT,
    clustered_forward as clustering,
    shadows,
};

struct ExaParams {
    // Linear rgb (×1.5 for legacy materials); a = opacity.
    diffuse: vec4<f32>,
    // Linear rgb × level; w = glossiness.
    specular: vec4<f32>,
    // Linear rgb; a = unlit opacity.
    emissive: vec4<f32>,
    flags: u32,
    pad0: u32,
    pad1: u32,
    pad2: u32,
    gi_origin: vec4<f32>,
    gi_extent: vec4<f32>,
    // Parallax scale per UV axis (xy), displacement-shadow step (z), bias (w).
    displacement: vec4<f32>,
}

const DIFFUSE_MAP: u32 = 1u;
const NORMAL_MAP: u32 = 2u;
const SPECULAR_MAP: u32 = 4u;
const EMISSIVE_MAP: u32 = 8u;
const OPACITY_MAP: u32 = 16u;
const LIT: u32 = 32u;
const PACKED_SPECULAR: u32 = 64u;
const TWO_SIDED: u32 = 128u;
const ENVIRONMENT: u32 = 256u;
const DIRT_MAP: u32 = 512u;
const GLOSS_MAP: u32 = 1024u;
const GI: u32 = 2048u;
const HEIGHT_MAP: u32 = 4096u;
const DISPLACEMENT_SHADOWS: u32 = 8192u;
// Displacement-shadow taps toward each light: four per sub-sample over the
// original's four supersampling passes.
const SHADOW_STEPS: i32 = 16;
// The original's texture LOD bias at its top texture quality with 4×
// supersampling, −0.75 × (4 − 1)/3, on colour textures only (not normal,
// gloss or packed specular maps). Rules: docs/RENDERING.md (Textures).
const TEXTURE_BIAS: f32 = -0.75;
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

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> material: ExaParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var diffuse_map: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var diffuse_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var normal_map: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var normal_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(105) var specular_map: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(106) var specular_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(107) var emissive_map: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(108) var emissive_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(109) var opacity_map: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(110) var opacity_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(111) var environment_map: texture_cube<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(112) var environment_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(113) var dirt_map: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(114) var dirt_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(115) var gloss_map: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(116) var gloss_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(117) var gi_volume: texture_3d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(118) var gi_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(119) var height_map: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(120) var height_sampler: sampler;

fn height(uv: vec2<f32>) -> f32 {
    return textureSample(height_map, height_sampler, uv).r;
}

// The height map's shadow on one light (direction l): taps marched toward
// the light in the tangent plane; lit where the surface stands above them.
fn displacement_shadow(uv: vec2<f32>, l: vec3<f32>, ax: vec3<f32>, ay: vec3<f32>, n: vec3<f32>, h0: f32) -> f32 {
    // Tangent-space light; its y runs along +v.
    let t = vec3(dot(l, ax), -dot(l, ay), dot(l, n));
    let a = material.displacement.z * (2.0 - t.z);
    var lit = 0.0;
    for (var j = 1; j <= SHADOW_STEPS; j++) {
        let h = height(uv + t.xy * a * f32(j));
        lit += clamp((h0 + 0.1 * t.z - h) * 20.0, 0.0, 1.0);
    }
    return lit / f32(SHADOW_STEPS);
}

// The GI volume around a surface point: four taps 20 cm out along the
// normal, spread across the tangent plane; light and openness averaged.
fn gi_sample(p: vec3<f32>, n: vec3<f32>) -> vec4<f32> {
    let a = select(vec3(0.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0), abs(n.y) > 0.9);
    let ax = normalize(cross(n, a));
    let ay = cross(n, ax);
    var sum = vec4(0.0);
    for (var i = 0; i < 4; i++) {
        let sx = select(-0.5, 0.5, (i & 1) == 1);
        let sy = select(-0.5, 0.5, (i & 2) == 2);
        let d = n + ax * sx + ay * sy;
        let w = (p + d * 0.2 - material.gi_origin.xyz) / material.gi_extent.xyz;
        let t = textureSampleLevel(gi_volume, gi_sampler, w.xzy, 0.0);
        sum += t;
    }
    return sum * 0.25;
}

fn has(bit: u32) -> bool {
    return (material.flags & bit) != 0u;
}

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

// Diffuse (x) and specular (rgb in the second) from one light of colour c.
struct Lit {
    diffuse: vec3<f32>,
    specular: vec3<f32>,
}

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

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var out: FragmentOutput;
    var uv = vec2(0.0);
@if(VERTEX_UVS_A) {
    uv = in.uv;
}
    let p = in.world_position.xyz;
    let v = normalize(view_bindings::view.world_position.xyz - p);
    // Parallax: the view in tangent space shifts the texture coordinates
    // by the height map, in four accumulating steps; every map then reads
    // at the shifted coordinates.
    var displaced = false;
    var dsp_val = 0.0;
    var ax = vec3(1.0, 0.0, 0.0);
    var ay = vec3(0.0, 0.0, 1.0);
    let ns = normalize(in.world_normal);
@if(VERTEX_TANGENTS) {
    if has(HEIGHT_MAP) {
        let tg = normalize(in.world_tangent.xyz);
        let btng = normalize(cross(tg, ns) * in.world_tangent.w);
        let eye = -v;
        let vt = vec3(dot(eye, tg), dot(eye, btng), dot(eye, ns));
        let dsp = vt.xy * material.displacement.xy * (4.0 + 3.0 * vt.z);
        for (var k = 0; k < 4; k++) {
            dsp_val = height(uv) - 0.5;
            uv -= dsp * dsp_val;
        }
        ax = normalize(in.world_tangent.xyz - ns * dot(in.world_tangent.xyz, ns));
        ay = cross(ns, ax) * in.world_tangent.w;
        displaced = true;
    }
}
    var dif_tex = vec4(1.0);
    if has(DIFFUSE_MAP) {
        dif_tex = textureSampleBias(diffuse_map, diffuse_sampler, uv, TEXTURE_BIAS);
    }
    var alpha = material.diffuse.a * dif_tex.a;
    if has(OPACITY_MAP) {
        let o = textureSampleBias(opacity_map, opacity_sampler, uv, TEXTURE_BIAS).a;
        alpha = material.diffuse.a * o * o;
    }

    // Unlit: emissive only.
    if !has(LIT) {
        var e = material.emissive.rgb;
        if has(EMISSIVE_MAP) {
            e *= textureSampleBias(emissive_map, emissive_sampler, uv, TEXTURE_BIAS).rgb;
        }
        var a = material.emissive.a;
        if has(OPACITY_MAP) {
            a *= textureSampleBias(opacity_map, opacity_sampler, uv, TEXTURE_BIAS).a;
        }
        out.color = vec4(e, a);
        return out;
    }

    var n = normalize(in.world_normal);
    if !is_front {
        n = -n;
    }
@if(VERTEX_TANGENTS) {
    if has(NORMAL_MAP) {
        let t = normalize(in.world_tangent.xyz - n * dot(in.world_tangent.xyz, n));
        let b = cross(n, t) * in.world_tangent.w;
        // The original's bitangent runs along −v of the stored UVs, as
        // Bevy's (its MikkTSpace sign is flipped), so green maps as stored.
        let xy = textureSample(normal_map, normal_sampler, uv).rg * 2.0 - 1.0;
        let z = sqrt(max(1.0 - dot(xy, xy), 0.0));
        n = normalize(t * xy.x + b * xy.y + n * z);
    }
}
    var s: Surface;
    s.n = n;
    s.v = v;
    s.ndv = abs(dot(v, n));
    var dif_col = material.diffuse.rgb * dif_tex.rgb;
    var cavity = 1.0;
    s.rough = 1.0 - material.specular.w;
    s.f0 = material.specular.rgb;
    s.legacy = !has(PACKED_SPECULAR);
    if has(SPECULAR_MAP) {
        let rgm = textureSampleBias(specular_map, specular_sampler, uv, select(TEXTURE_BIAS, 0.0, has(PACKED_SPECULAR)));
        if has(PACKED_SPECULAR) {
            let metal = rgm.b;
            s.f0 = mix(vec3(0.04), dif_tex.rgb * material.specular.rgb, metal);
            dif_col *= 1.0 - metal;
            s.rough = 1.0 - material.specular.w * rgm.g;
            cavity = rgm.r;
        } else {
            s.f0 = material.specular.rgb * rgm.rgb;
        }
    }
    if has(GLOSS_MAP) && !has(SPECULAR_MAP) {
        s.rough = 1.0 - material.specular.w * textureSample(gloss_map, gloss_sampler, uv).r;
    }
    s.rough = clamp(s.rough, 0.02, 1.0);

    var diffuse = vec3(0.0);
    var specular = vec3(0.0);
    let self_shadowed = displaced && has(DISPLACEMENT_SHADOWS);
    let h0 = dsp_val + material.displacement.w + 0.5;
    let receiver = (mesh[in.instance_index].flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u;

    // Point lights in this fragment's cluster.
    let view_z = dot(vec4<f32>(
        view_bindings::view.view_from_world[0].z,
        view_bindings::view.view_from_world[1].z,
        view_bindings::view.view_from_world[2].z,
        view_bindings::view.view_from_world[3].z,
    ), in.world_position);
    let ortho = view_bindings::view.clip_from_view[3].w == 1.0;
    let cluster = clustering::view_fragment_cluster_index(in.position.xy, view_z, ortho);
    let ranges = clustering::unpack_clusterable_object_index_ranges(cluster);
    for (var i = ranges.first_point_light_index_offset; i < ranges.first_spot_light_index_offset; i++) {
        let id = clustering::get_clusterable_object_id(i);
        let lt = &view_bindings::clustered_lights.data[id];
        let to = (*lt).position_radius.xyz - p;
        let d = length(to);
        let atn = pow(max(1.0 - d / max((*lt).range, 1e-3), 0.0), 4.0);
        if atn <= 0.0 {
            continue;
        }
        var shadow = 1.0;
        if receiver && ((*lt).flags & mesh_view_types::POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u {
            shadow = shadows::fetch_point_shadow(id, in.world_position, in.world_normal, in.position.xy);
        }
        let l = to / max(d, 1e-4);
        if self_shadowed {
            shadow *= displacement_shadow(uv, l, ax, ay, ns, h0);
        }
        let lit = light(s, l, (*lt).color_inverse_square_range.rgb * atn * shadow);
        diffuse += lit.diffuse;
        specular += lit.specular;
    }
    // The sun.
    for (var i = 0u; i < view_bindings::lights.n_directional_lights; i++) {
        let sun = &view_bindings::lights.directional_lights[i];
        var shadow = 1.0;
        if receiver && ((*sun).flags & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u {
            shadow = shadows::fetch_directional_shadow(i, in.world_position, in.world_normal, view_z, in.position.xy);
        }
        if self_shadowed {
            shadow *= displacement_shadow(uv, (*sun).direction_to_light, ax, ay, ns, h0);
        }
        let lit = light(s, (*sun).direction_to_light, (*sun).color.rgb / SUN_LUX * shadow);
        diffuse += lit.diffuse;
        specular += lit.specular;
    }

    // Ambient: the environment cube's blurred colour around the normal ×
    // the level's ambient × 16; reflections sample it by gloss.
    let level = view_bindings::lights.ambient_color.rgb;
    var ambient = level * ENVIRONMENT_FALLBACK;
    var mirrored = vec3(ENVIRONMENT_FALLBACK);
    let r = reflect(-v, n);
    let k = (s.rough + 1.0) * (s.rough + 1.0) / 8.0;
    let gv = s.ndv / (s.ndv * (1.0 - k) + k);
    var reflected: vec3<f32>;
    let gw = (p - material.gi_origin.xyz) / material.gi_extent.xyz;
    if has(GI) && all(gw >= vec3(0.0)) && all(gw <= vec3(1.0)) {
        // Voxel GI: the volume's light, lifted where faint, × the level
        // ambient × 15 × openness; reflections take the cube × that.
        let g = gi_sample(p, n);
        let a = length(g.rgb);
        let b = (1.0 - a) * (1.0 - a) * 0.03 / (a + 0.016) + 0.47 * a + 0.53;
        ambient = g.rgb * level / AMBIENT_SCALE * GI_SCALE * b * g.a;
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
    // Parallax without displacement shadows darkens the ambient in hollows.
    if displaced && !self_shadowed {
        ambient *= min(dsp_val + 1.2, 1.0);
    }
    var colour = (diffuse + ambient) * dif_col;
    if s.legacy {
        // Older program: (lights + reflection) × specular colour × a soft
        // Fresnel between 0.5 and 1.
        let spf = 0.5 + 0.5 * exp2((-5.55473 * s.ndv - 6.98316) * s.ndv);
        colour += (specular + reflected) * s.f0 * spf;
    } else {
        specular += reflected * schlick(s.f0, s.ndv);
        colour += specular * cavity;
    }

    var e = material.emissive.rgb;
    if has(EMISSIVE_MAP) {
        e *= textureSampleBias(emissive_map, emissive_sampler, uv, TEXTURE_BIAS).rgb;
    }
    colour += e;
    out.color = vec4(colour, alpha);
    return out;
}
