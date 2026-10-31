// The teleport bubble: the original's `effecttele_v` / `effecttele_f` over
// the bubble's sphere (the unit icosahedron subdivided three times, its
// points at 120 cm, every face drawn reversed and then as built). The mesh
// carries the original's centimetre points and the entity's transform the
// bubble's place with the centimetre-to-metre scale, so `vPos` is the
// original's attribute. The vertex stage scales it by `Scale` (the swell),
// places it, and hands on the scaled point (`vrPos`) and the unit vector
// from `ViewPos` (the camera in the bubble's frame, cm) to it (`vrVwr`),
// interpolated as the original's varyings are. The fragment stage is
// `effecttele_f` as shipped: three planar `EmiTex` lookups (the
// `_ForceNoise` picture) blended by the squared normal, twice, the rolling
// bands of `Anim` and `Pulse`, the `Fade` front and the rim. The program
// declares `TexOfs` but never reads it. The colour goes out with alpha 0:
// under the `_TeleBubble01` blend word 0x163 (ONE, ONE_MINUS_SRC_ALPHA,
// Bevy's premultiplied blend) the bubble only adds light. The output is
// clamped to [0, 1] as the original's 8-bit target clamps it.
// Rules: exa_render::force_effects and docs/RENDERING.md (Force effects).

import bevy_pbr::render::{
    mesh_functions,
    view_transformations::position_world_to_clip,
};

struct TeleBubbleParams {
    // `ViewPos`: the camera in the bubble's frame (cm); w unused.
    view_pos: vec4<f32>,
    // `TexPos` in xy; zw unused.
    tex_pos: vec4<f32>,
    // `Scale`: 1, or `1 + 100·s⁴` once the charge passes 0.8.
    scale: f32,
    // `Fade`.
    fade: f32,
    // `Pulse`: the charge.
    pulse: f32,
    // `Anim`: the rolling texture offset, 0..1.
    anim: f32,
    // `Colr`: 1, or `1 − s` once the charge passes 0.8.
    colr: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> bubble: TeleBubbleParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var emi_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var emi_sampler: sampler;

struct BubbleVertex {
    @builtin(instance_index) instance_index: u32,
    // `vPos` (cm).
    @location(0) position: vec3<f32>,
}

struct BubbleVaryings {
    @builtin(position) position: vec4<f32>,
    @location(0) vr_vwr: vec3<f32>,
    @location(1) vr_pos: vec3<f32>,
}

@vertex
fn vertex(vertex_in: BubbleVertex) -> BubbleVaryings {
    var out: BubbleVaryings;
    let s_pos = vertex_in.position * bubble.scale;
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
    let world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(s_pos, 1.0));
    out.position = position_world_to_clip(world.xyz);
    out.vr_pos = s_pos;
    out.vr_vwr = normalize(s_pos - bubble.view_pos.xyz);
    return out;
}

fn emi(tc: vec2<f32>) -> f32 {
    return textureSample(emi_texture, emi_sampler, tc).r;
}

@fragment
fn fragment(in: BubbleVaryings) -> @location(0) vec4<f32> {
    let fade = bubble.fade;
    let pulse = bubble.pulse;
    let tex_pos = bubble.tex_pos.xy;
    let nrm = normalize(in.vr_pos);
    var va = dot(in.vr_vwr, nrm);
    var f = nrm.y * 0.5 + 0.5;
    let th = f;
    var a = (1.0 - th * 0.5) * 0.007;
    var tcz = in.vr_pos.xy * a;
    var tcy = in.vr_pos.xz * a;
    var tcx = in.vr_pos.yz * a;
    tcx.x = tcx.x + sign(nrm.x) * 0.333;
    tcy.y = tcy.y + sign(nrm.y) * 0.333;
    tcz.x = tcz.x - sign(nrm.z) * 0.333;
    var tbw = abs(nrm * nrm);
    tbw = tbw / (tbw.x + tbw.y + tbw.z);
    var to = tcz * 0.5 + tex_pos;
    var i1 = emi(to) * tbw.z;
    to = tcx * 0.5 + tex_pos;
    i1 = i1 + emi(to) * tbw.x;
    to = tcy * 0.5 + tex_pos;
    i1 = i1 + emi(to) * tbw.y;
    i1 = i1 * i1 * 2.0;
    var noise = i1;
    tcz += 0.333 + tex_pos;
    var i0 = emi(tcz) * tbw.z;
    tcx += 0.333 + tex_pos;
    i0 = i0 + emi(tcx) * tbw.x;
    tcy += 0.333 + tex_pos;
    i0 = i0 + emi(tcy) * tbw.y;
    var b = abs(1.0 - abs(i0 - 0.5) * 2.0);
    noise = noise * (0.5 + b * 0.6);
    var d = abs(nrm.y) - bubble.anim;
    d = d - round(d);
    d = d * d * 4.0 * pulse;
    a = 1.0 - abs(th * 2.0 - 1.0);
    a = mix(a, 1.0 - noise, 0.4);
    f = max(fade, 0.0);
    a = pow(a, 16.0 * f);
    var c = a;
    a = 1.0 - abs(0.5 - a) * 2.0;
    a = a * a;
    var oln = a;
    oln = oln + noise * d * 2.0;
    b = 0.5 + fade * 0.5;
    noise = noise * b + (1.0 - b) * 0.5;
    var srf = noise * round(c);
    srf = srf * (1.0 + abs(fade) + pulse);
    c = a;
    va = 1.0 - abs(va);
    va = va * va;
    va = va * va * 1.5;
    va = (0.75 + va * 3.0) * max(1.0 - va * va * 1.5, 0.0);
    a = fade * fade;
    va = va * (1.0 - a) + a;
    let fcol = vec3(0.40, 0.42, 0.5);
    a = min((1.0 - f) * 8.0, 1.0) * bubble.colr;
    srf = srf * a;
    oln = oln * a;
    var col = fcol * (oln * 1.0 + srf * va * 2.0);
    col = col * col;
    return clamp(vec4(col * col, 0.0), vec4(0.0), vec4(1.0));
}
