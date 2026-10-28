// The original's surface shading (its `shadeopt` program) for models, on
// Bevy's lights; the lighting itself is shared (`shading.wesl`). Rules:
// docs/RENDERING.md.

import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    mesh_bindings::mesh,
    mesh_view_bindings as view_bindings,
    mesh_view_types,
    mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT,
    clustered_forward as clustering,
    shadows,
};
import exa_render::shading::{
    Surface, Lit, taps, lit_by, light_shadow, light, light_skin, schlick, view_depth, vertical_shaping,
    gi_sample, in_gi, gi_ambient,
    REFLECTION_STRENGTH, ENVIRONMENT_FALLBACK, AMBIENT_SCALE, SUN_LUX,
};

struct ExaParams {
    // Linear rgb (×1.5 for legacy materials); a = opacity.
    diffuse: vec4<f32>,
    // Linear rgb × level; w = glossiness.
    specular: vec4<f32>,
    // Linear rgb (×2 lit with an emissive map); a = unlit opacity.
    emissive: vec4<f32>,
    flags: u32,
    // The LOD bias on colour-texture samples (`ExaParams::texture_bias`).
    texture_bias: f32,
    pad1: u32,
    pad2: u32,
    gi_origin: vec4<f32>,
    gi_extent: vec4<f32>,
    // Parallax scale per UV axis (xy), displacement-shadow step (z), bias (w).
    displacement: vec4<f32>,
    // Skin: the scattered share of the diffuse colour (rgb) and the blur (w).
    subsurface: vec4<f32>,
    // Opacity fall-off: alpha += 2^((x − N·V)·y)·z.
    opacity_falloff: vec4<f32>,
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
const DEBUG_GI: u32 = 16384u;
const SPLIT_TAPS: u32 = 32768u;
const SUBSURFACE: u32 = 65536u;
const FADE_TO_BLACK: u32 = 131072u;
const TINT_AS_SRGB: u32 = 262144u;
// The horizontal distance from the origin (m) at which the fade to black
// reaches black (`exa_render::material::FADE_RADIUS`).
const FADE_RADIUS: f32 = 2.0;
// The mip level of the diffuse map the light scattered under the skin
// takes its colour from (the original's subsurface program).
const SCATTER_COLOUR_LOD: f32 = 1.5;
const ADDITIVE: u32 = 524288u;
const OPACITY_FALLOFF: u32 = 1048576u;
// Blood (the original's dirt mode 4) as its apparel and item, skin and hair
// programs show it (`exa_render::material::BloodProgram`).
const BLOOD: u32 = 2097152u;
const BLOOD_SKIN: u32 = 4194304u;
const BLOOD_HAIR: u32 = 8388608u;
// The diffuse map is a single-channel BC4 image (the original's luminance
// LATC upload): its red channel is the whole colour, with no sRGB decode.
const GREY_DIFFUSE: u32 = 16777216u;
// The diffuse map skips the LOD bias (a record with a secondary colour).
const SKIP_DIFFUSE_BIAS: u32 = 33554432u;
// The specular and emissive maps are single-channel BC4 images (the same
// luminance LATC upload): the shader repeats the red channel over rgb.
const GREY_SPECULAR: u32 = 67108864u;
const GREY_EMISSIVE: u32 = 134217728u;
const BLOOD_TINT = vec3(0.6, 0.03, 0.0);
const HAIR_BLOOD_TINT = vec3(0.45, 0.03, 0.02);
const SKIN_ALPHA_FLOOR: f32 = 0.02;
const CLOTH_BLOOD_GAIN: f32 = 1.3;
const WET_ROUGHNESS: f32 = 0.2;
// Displacement-shadow taps toward each light: four per sub-sample over the
// original's four supersampling passes.
const SHADOW_STEPS: i32 = 16;
// The original's texture LOD bias at its top texture quality with 4×
// supersampling, −0.75 × (4 − 1)/3, on colour textures only (not normal,
// gloss or packed specular maps). Rules: docs/RENDERING.md (Textures).

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

fn has(bit: u32) -> bool {
    return (material.flags & bit) != 0u;
}

// The sRGB curve's decoding, per channel.
fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3(2.4));
    return select(hi, lo, c <= vec3(0.04045));
}

// The colour as drawn. Alpha is clamped to [0, 1] as the original's 8-bit
// target clamps a fragment before blending; the opacity fall-off takes it
// past 1 at a glass's outline, which on Exarust's float target would
// subtract what is behind. Additive surfaces (the original's SRC_ALPHA, ONE)
// go through Bevy's premultiplied blending (ONE, ONE_MINUS_SRC_ALPHA) as
// their colour, clamped to 1 likewise, times alpha with no alpha, which
// leaves what is behind them whole.
fn blended(rgb: vec3<f32>, alpha: f32) -> vec4<f32> {
    let a = clamp(alpha, 0.0, 1.0);
    if has(ADDITIVE) {
        return vec4(min(rgb, vec3(1.0)) * a, 0.0);
    }
    return vec4(rgb, a);
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
        let dif_bias = select(material.texture_bias, 0.0, has(SKIP_DIFFUSE_BIAS));
        dif_tex = textureSampleBias(diffuse_map, diffuse_sampler, uv, dif_bias);
        if has(GREY_DIFFUSE) {
            // A BC4 diffuse is the original's single-channel luminance
            // format: one grey channel, never sRGB-decoded by the hardware.
            dif_tex = vec4(dif_tex.rrr, dif_tex.a);
        }
    }
    var alpha = material.diffuse.a * dif_tex.a;
    if has(OPACITY_MAP) {
        let o = textureSampleBias(opacity_map, opacity_sampler, uv, material.texture_bias).a;
        alpha = material.diffuse.a * o * o;
    }

    // Unlit: emissive only. An emissive map's glow uses its alpha unless the
    // fall-off sets the alpha; a bare opacity map scales it. The original
    // never combines the two maps on an unlit surface.
    if !has(LIT) {
        var e = material.emissive.rgb;
        var a = material.emissive.a;
        if has(EMISSIVE_MAP) {
            var em = textureSampleBias(emissive_map, emissive_sampler, uv, material.texture_bias);
            if has(GREY_EMISSIVE) {
                em = vec4(em.rrr, em.a);
            }
            e *= em.rgb;
            if has(OPACITY_FALLOFF) {
                // The opacity fall-off with the view angle: a salve's glow is
                // strong where it faces the eye and fades toward its outline.
                let f = material.opacity_falloff;
                a += exp2((f.x - abs(dot(v, ns))) * f.y) * f.z;
            } else {
                a *= em.a;
            }
        } else if has(OPACITY_MAP) {
            a *= textureSampleBias(opacity_map, opacity_sampler, uv, material.texture_bias).a;
        }
        out.color = blended(e, a);
        return out;
    }

    // Skin: the light scattered under it reads a blurred normal.
    let skin = has(SUBSURFACE);
    let blur = material.subsurface.w;
    let geom = ns;
    let back_face = has(TWO_SIDED) && !is_front;
    var n = geom;
    var sn = geom;
@if(VERTEX_TANGENTS) {
    if has(NORMAL_MAP) {
        let t = normalize(in.world_tangent.xyz - geom * dot(in.world_tangent.xyz, geom));
        let b = cross(geom, t) * in.world_tangent.w;
        // The original's bitangent runs along −v of the stored UVs, as
        // Bevy's (its MikkTSpace sign is flipped), so green maps as stored.
        let xy = textureSample(normal_map, normal_sampler, uv).rg * 2.0 - 1.0;
        let z = sqrt(max(1.0 - dot(xy, xy), 0.0));
        // A two-sided back face flips only the map's z, not the tangent
        // frame, as the original's normal include does.
        n = normalize(t * xy.x + b * xy.y + geom * select(z, -z, back_face));
        if skin {
            let bxy = textureSampleLevel(normal_map, normal_sampler, uv, blur).rg * 2.0 - 1.0;
            let bz = sqrt(max(1.0 - dot(bxy, bxy), 0.0));
            sn = normalize(t * bxy.x + b * bxy.y + geom * select(bz, -bz, back_face));
        } else {
            sn = n;
        }
    } else if back_face {
        n = -geom;
        sn = n;
    }
}
@else {
    if back_face {
        n = -geom;
        sn = n;
    }
}
    let shadow_normal = select(geom, -geom, back_face);
    var s: Surface;
    s.n = n;
    s.v = v;
    s.ndv = abs(dot(v, n));
    var dif_col = material.diffuse.rgb * dif_tex.rgb;
    // The colour the light scattered under the skin takes: the diffuse map
    // blurred.
    var scatter_col = material.diffuse.rgb;
    if skin && has(DIFFUSE_MAP) {
        let blurred = textureSampleLevel(diffuse_map, diffuse_sampler, uv, SCATTER_COLOUR_LOD);
        scatter_col *= select(blurred.rgb, blurred.rrr, has(GREY_DIFFUSE));
    }
    // A map tinted in sRGB space (a character's skin, which the original
    // bakes from undecoded texels and reads back as sRGB): decode the
    // product.
    if has(TINT_AS_SRGB) {
        dif_col = srgb_to_linear(dif_col);
        scatter_col = srgb_to_linear(scatter_col);
    }
    var cavity = 1.0;
    s.rough = 1.0 - material.specular.w;
    s.f0 = material.specular.rgb;
    s.legacy = !has(PACKED_SPECULAR);
    if has(SPECULAR_MAP) {
        let bias = select(material.texture_bias, 0.0, has(PACKED_SPECULAR));
        var rgm = textureSampleBias(specular_map, specular_sampler, uv, bias);
        if has(GREY_SPECULAR) {
            rgm = vec4(rgm.rrr, rgm.a);
        }
        if has(PACKED_SPECULAR) {
            let metal = rgm.b;
            if has(DIFFUSE_MAP) {
                s.f0 = mix(vec3(0.04), dif_tex.rgb * material.specular.rgb, metal);
            } else {
                // Without a diffuse map the original mixes the other way:
                // the specular colour scaled by the metal amount, toward
                // 0.04.
                s.f0 = mix(material.diffuse.rgb * material.specular.rgb, vec3(0.04), metal);
            }
            dif_col *= 1.0 - metal;
            scatter_col *= 1.0 - metal;
            s.rough = 1.0 - material.specular.w * rgm.g;
            cavity = rgm.r;
        } else {
            s.f0 = material.specular.rgb * rgm.rgb;
        }
    }
    if has(GLOSS_MAP) && !has(SPECULAR_MAP) {
        s.rough = 1.0 - material.specular.w * textureSample(gloss_map, gloss_sampler, uv).r;
    }
    // Blood: wounds lower the mesh's vertex alpha (1 clean); where it falls
    // below the tiling speckle (the dirt map, read at UV×2, at UV×1 for
    // hair) the amount da = clamp((a − α)/a·k, 0, 1) darkens the diffuse
    // colour toward dark red and wets the surface. Meshes without a colour
    // stream have never been wounded.
@if(VERTEX_COLORS) {
    if has(BLOOD) || has(BLOOD_SKIN) || has(BLOOD_HAIR) {
        let hair = has(BLOOD_HAIR);
        let a = textureSample(dirt_map, dirt_sampler, uv * select(2.0, 1.0, hair)).r;
        var va = in.color.a;
        var gain = 1.0;
        if has(BLOOD_SKIN) {
            va = max(va, SKIN_ALPHA_FLOOR);
        } else if !hair {
            gain = CLOTH_BLOOD_GAIN;
        }
        let da = clamp((a - va) / max(a, 1e-5) * gain, 0.0, 1.0);
        let dark = 1.0 - 0.3 * da;
        var bld: vec3<f32>;
        if has(BLOOD_SKIN) {
            s.rough = max(s.rough - da * (0.1 + 0.6 * a), 0.1);
            bld = BLOOD_TINT * (1.0 - da * da * 0.5) * dark;
        } else if hair {
            s.rough = mix(s.rough, WET_ROUGHNESS, da * da * (0.7 * a + 0.3));
            bld = HAIR_BLOOD_TINT * dark;
        } else {
            s.rough = mix(s.rough, WET_ROUGHNESS, da * da * (0.5 * a + 0.5));
            bld = BLOOD_TINT * dark;
        }
        bld = clamp(bld + vec3(1.0 - da), vec3(0.0), vec3(1.0));
        // Both the plain and the scattered share of the skin.
        dif_col *= bld;
        scatter_col *= bld;
    }
}
    s.rough = clamp(s.rough, 0.02, 1.0);
    // The skin program sets the fragment's alpha to the material opacity at
    // the end, ignoring the diffuse map's alpha.
    if skin {
        alpha = material.diffuse.a;
    }

    var diffuse = vec3(0.0);
    var scatter = vec3(0.0);
    var specular = vec3(0.0);
    let self_shadowed = displaced && has(DISPLACEMENT_SHADOWS);
    let h0 = dsp_val + material.displacement.w + 0.5;
    let receiver = (mesh[in.instance_index].flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u;

    let disc = taps(in.position.xy, has(SPLIT_TAPS));
    let tag = mesh[in.instance_index].tag;
    // Point lights in this fragment's cluster.
    let view_z = view_depth(in.world_position);
    let ortho = view_bindings::view.clip_from_view[3].w == 1.0;
    let cluster = clustering::view_fragment_cluster_index(in.position.xy, view_z, ortho);
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
        var shadow = light_shadow(id, p, n, disc, tag, receiver);
        let l = to / max(d, 1e-4);
        if self_shadowed {
            shadow *= displacement_shadow(uv, l, ax, ay, shadow_normal, h0);
        }
        let colour = (*lt).color_inverse_square_range.rgb * atn * shadow;
        if skin {
            let lit = light_skin(s, sn, blur, l, colour);
            diffuse += lit.diffuse;
            scatter += lit.scatter;
            specular += lit.specular;
            continue;
        }
        let lit = light(s, l, colour);
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
            shadow *= displacement_shadow(uv, (*sun).direction_to_light, ax, ay, shadow_normal, h0);
        }
        let colour = (*sun).color.rgb / SUN_LUX * shadow;
        if skin {
            let lit = light_skin(s, sn, blur, (*sun).direction_to_light, colour);
            diffuse += lit.diffuse;
            scatter += lit.scatter;
            specular += lit.specular;
            continue;
        }
        let lit = light(s, (*sun).direction_to_light, colour);
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
    let gi_origin = material.gi_origin.xyz;
    let gi_extent = material.gi_extent.xyz;
    if has(GI) && in_gi(gi_origin, gi_extent, p) {
        // Voxel GI: the volume's light, lifted where faint, × the level
        // ambient × 15 × openness; reflections take the cube × that.
        let g = gi_sample(gi_volume, gi_sampler, gi_origin, gi_extent, p, n, disc);
        if has(DEBUG_GI) {
            out.color = vec4(g.rgb * 10.0, 1.0);
            return out;
        }
        ambient = gi_ambient(g, level);
        if has(ENVIRONMENT) {
            mirrored = textureSampleLevel(environment_map, environment_sampler, r, max(s.rough - 0.05, 0.0) * 8.0).rgb;
            reflected = mirrored * ambient * REFLECTION_STRENGTH * 2.0 * gv;
        } else {
            // Without a reflection layer (`Mat.TexRef`) the specular GI is a
            // quarter of the ambient, not a cube sample.
            reflected = ambient * 0.25 * gv;
        }
    } else {
        // Outside the GI volume the cube lights and reflects every surface,
        // with or without a reflection layer of its own.
        ambient = textureSampleLevel(environment_map, environment_sampler, n, 5.0).rgb * level;
        mirrored = textureSampleLevel(environment_map, environment_sampler, r, max(s.rough - 0.1, 0.0) * 8.0).rgb;
        reflected = (mirrored + mirrored * mirrored) * level / AMBIENT_SCALE * REFLECTION_STRENGTH * gv;
    }
    // Parallax darkens the PBR program's ambient in hollows, with or without
    // the displacement shadows (`Mat.TexDsp`, independent of `Shd.DispSdw`;
    // the older program adds its ambient as it is).
    if displaced && !s.legacy {
        ambient *= min(dsp_val + 1.2, 1.0);
    }
    var colour = (diffuse + ambient) * dif_col;
    if skin {
        // The scattered share takes its own light; ambient reaches both
        // shares, so it weighs 1 in all.
        let sss = material.subsurface.rgb;
        colour = (diffuse + ambient) * (vec3(1.0) - sss) * dif_col + (scatter + ambient) * sss * scatter_col;
    }
    if s.legacy {
        // Older program: (lights + reflection) × specular colour × a soft
        // Fresnel between 0.5 and 1.
        let spf = 0.5 + 0.5 * exp2((-5.55473 * s.ndv - 6.98316) * s.ndv);
        colour += (specular + reflected) * s.f0 * spf;
    } else {
        specular += reflected * schlick(s.f0, s.ndv);
        colour += specular * cavity;
    }

    // The opacity fall-off, as on unlit surfaces but with the shaded normal,
    // and an emissive map's glow dimmed toward the outline by the fall-off's
    // amount, N·V²·z + 1 − z (1 without a fall-off). Only the PBR program
    // does either; the older one's lit path has neither.
    var fall = material.opacity_falloff;
    if s.legacy {
        fall = vec4(0.0);
    }
    if has(OPACITY_FALLOFF) && !s.legacy {
        alpha += exp2((fall.x - s.ndv) * fall.y) * fall.z;
    }
    var e = material.emissive.rgb;
    if has(EMISSIVE_MAP) {
        var em = textureSampleBias(emissive_map, emissive_sampler, uv, material.texture_bias).rgb;
        if has(GREY_EMISSIVE) {
            em = em.rrr;
        }
        e *= em * (s.ndv * s.ndv * fall.z + 1.0 - fall.z);
    }
    colour += e;
    // The character screen's fade: black by FADE_RADIUS from the origin.
    if has(FADE_TO_BLACK) {
        let a = min(length(p.xz) / FADE_RADIUS, 1.0);
        colour *= 1.0 - a * (2.0 - a);
    }
    out.color = blended(colour, alpha);
    return out;
}
