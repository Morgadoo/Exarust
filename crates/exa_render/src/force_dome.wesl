// The force domes: the original's `effectbarrier_v` with `effectbarrier_f`
// (the Barrier's cap) or, with `BUBBLE`, `effectbubble_f` (the Bubble's
// sphere). Both meshes are the subdivided icosahedron (the Barrier its top
// cap, y and z exchanged so the cap faces +z) with its points at the dome's
// radius in centimetres, every face drawn reversed and then as built. The
// entity's transform carries the node's frame (the Barrier turned to its
// caster's facing, the Bubble unturned) with the centimetre-to-metre scale,
// so `vPos` is the original's attribute. The vertex stage places it and
// hands on the point itself (`vrPos`) and the unit vector from `ViewPos`
// (the camera in the node's frame, cm) to it (`vrVwr`), interpolated as the
// original's varyings are. The fragment stages are the programs as
// shipped: `EmiTex` (the `_ForceNoise` picture) looked up once (the Barrier,
// over its cap's xy) or three times by the squared normal (the Bubble),
// twice; the eight ripple points of `ColPt`; the `Fade` the grow and shrink
// run through; the rim; and the violet colour ramp. Both programs declare
// `TexOfs` but neither reads it. The `_ForceBarrier01` record's blend word
// is 0x163 (ONE, ONE_MINUS_SRC_ALPHA), Bevy's premultiplied blend; the
// record is one-sided (flags 0x480008). The output is clamped to [0, 1] as
// the original's 8-bit target clamps it.
// Rules: exa_render::force_effects and docs/RENDERING.md (Force effects).

import bevy_pbr::render::{
    mesh_functions,
    view_transformations::position_world_to_clip,
};

struct ForceDomeParams {
    // `ViewPos`: the camera in the node's frame (cm); w unused.
    view_pos: vec4<f32>,
    // `TexPos` in xy; zw unused.
    tex_pos: vec4<f32>,
    // `ColPt`: the eight ripple points (cm, the node's frame), their shown
    // amount in w.
    colour_points: array<vec4<f32>, 8>,
    // `Fade`.
    fade: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> dome: ForceDomeParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var emi_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var emi_sampler: sampler;

struct DomeVertex {
    @builtin(instance_index) instance_index: u32,
    // `vPos` (cm).
    @location(0) position: vec3<f32>,
}

struct DomeVaryings {
    @builtin(position) position: vec4<f32>,
    @location(0) vr_vwr: vec3<f32>,
    @location(1) vr_pos: vec3<f32>,
}

@vertex
fn vertex(vertex_in: DomeVertex) -> DomeVaryings {
    var out: DomeVaryings;
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
    let world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(vertex_in.position, 1.0));
    out.position = position_world_to_clip(world.xyz);
    out.vr_pos = vertex_in.position;
    out.vr_vwr = normalize(vertex_in.position - dome.view_pos.xyz);
    return out;
}

fn emi(tc: vec2<f32>) -> f32 {
    return textureSample(emi_texture, emi_sampler, tc).r;
}

@fragment
fn fragment(in: DomeVaryings) -> @location(0) vec4<f32> {
    let fade = dome.fade;
    let tex_pos = dome.tex_pos.xy;
    let nrm = normalize(in.vr_pos);
    var va = dot(in.vr_vwr, nrm);
    var a = dot(in.vr_vwr.xy, in.vr_vwr.xy);
    var f = dot(in.vr_pos.xy, in.vr_pos.xy) * 0.000125;
    var cf = 0.0;
    var cp = vec3(0.0);
@if(!BUBBLE) {
    // effectbarrier_f
    f = max(1.0 - f * 0.7, 0.0);
    a = fade * 0.6;
    f = max(f - a * a * 2.0, 0.0);
    var th = f;
    var tc = in.vr_pos.xy * (1.0 - th * 0.5) * 0.005;
    var to = tc * 0.75 + tex_pos;
    var i1 = emi(to);
    var cm = 1.0;
    for (var i = 0; i < 8; i++) {
        cp = in.vr_pos - dome.colour_points[i].xyz;
        a = dot(cp, cp);
        a = max(1.0 - a * 0.0005, 0.0);
        cf = cf + a * a * dome.colour_points[i].w * 0.5;
        cm = cm + a * dome.colour_points[i].w * 0.1;
    }
    i1 = i1 + min(cf, 1.8);
    var ta = i1;
    i1 = i1 + max(2.0 - f * 4.0, 0.0);
    i1 = max(i1 - f * f * 0.25, 0.0);
    i1 = (i1 * (2.0 - i1) + i1) * 0.5;
    a = abs(i1 - 0.5) * 2.0;
    var b = abs(1.0 - abs(a - 0.5) * 4.0);
    b = 1.0 - abs(0.5 - b) * (0.5 + i1);
    b = b * (1.0 - a);
    i1 = (b * b + b) * 0.55;
    i1 = i1 * min(cm, 1.15);
    tc = tc + (0.333 + tex_pos);
    var i0 = emi(tc);
    i0 = i0 * th * th * 1.5;
    a = abs(i0 - 0.5) * 2.0;
    b = abs(1.0 - abs(a - 0.5) * 4.0);
    b = 1.0 - abs(0.5 - b) * 2.0;
    i0 = b * (1.0 - a);
    a = i0 * i1 * 2.0;
    b = 1.0 - 2.0 * (1.0 - i0) * (1.0 - i1);
    va = 1.1 - abs(va) - b * b * 0.4;
    va = va * va;
    va = (1.0 + va * 2.0) * max(1.0 - va * va, 0.0);
    var c = clamp((0.5 - i1) * 10.0 + 0.5, 0.0, 1.0);
    a = mix(b, a, c);
    a = a * 0.5 + 0.5;
    a = (a * a + a) * 0.5;
    th = 1.0 - th;
    b = va * (0.5 + th * th * 2.0);
    f = 1.0 - f;
    f = f * f;
    ta = (1.0 - ta) - f * f * 6.0;
    ta = ta * b * b * 0.5 * (1.0 - fade * fade);
    let alpha = ta;
    a = max(a - f * f * 2.5, 0.0);
    b = max(a - 0.5, 0.0);
    b = b * b;
    b = b * b * 4.0;
    a = a * 0.4 * (1.0 - b) + max(1.0 / (1.3 - a) - 1.0, 0.0) * b;
    a = a * (1.0 - fade * fade * 0.5);
    let fcol = vec3(0.45, 0.33, 0.95);
    c = clamp((0.5 - a) * 10.0 + 0.5, 0.0, 1.0);
    var col = vec3(1.0) - (1.0 - fcol) * (1.0 - a) * 2.0;
    col = mix(col, fcol * a * 2.0, vec3(c));
    col = col * va * 2.0;
    col = col * (0.5 + th * th * 2.0);
    col = max(col - 0.1, vec3(0.0));
    return clamp(vec4(col * col * 0.6, alpha), vec4(0.0), vec4(1.0));
}
@if(BUBBLE) {
    // effectbubble_f
    f = max(1.0 - f * 0.4, 0.0);
    a = fade * 0.6;
    f = max(f - a * a * 2.0, 0.0);
    a = (1.5 + nrm.y) * fade;
    f = clamp(1.0 - a, 0.0, 1.0) * 0.9;
    var th = f;
    a = (1.0 - th * 0.5) * 0.007;
    var tcz = in.vr_pos.xy * a;
    var tcy = in.vr_pos.xz * a;
    var tcx = in.vr_pos.yz * a;
    tcx.x = tcx.x + sign(nrm.x) * 0.333;
    tcy.y = tcy.y + sign(nrm.y) * 0.333;
    tcz.x = tcz.x - sign(nrm.z) * 0.333;
    var tbw = abs(nrm * nrm);
    tbw = tbw / (tbw.x + tbw.y + tbw.z);
    var to = tcz * 0.75 + tex_pos;
    var i1 = emi(to) * tbw.z;
    to = tcx * 0.75 + tex_pos;
    i1 = i1 + emi(to) * tbw.x;
    to = tcy * 0.75 + tex_pos;
    i1 = i1 + emi(to) * tbw.y;
    var cm = 0.0;
    for (var i = 0; i < 8; i++) {
        cp = in.vr_pos - dome.colour_points[i].xyz;
        a = dot(cp, cp);
        a = max(1.0 - a * 0.0005, 0.0);
        cf = cf + a * a * dome.colour_points[i].w * 0.2;
        cm = cm + a * dome.colour_points[i].w * 0.02;
    }
    cm = cm * abs(va);
    i1 = i1 + min(cf, 1.8);
    var ta = i1;
    i1 = i1 + max(2.0 - f * 4.0, 0.0);
    i1 = max(i1 - f * f * 0.25, 0.0);
    i1 = (i1 * (2.0 - i1) + i1) * 0.5;
    a = abs(i1 - 0.5) * 2.0;
    var b = abs(1.0 - abs(a - 0.5) * 4.0);
    b = 1.0 - abs(0.5 - b) * (0.5 + i1);
    b = b * (1.0 - a);
    i1 = (b * b + b) * 0.55;
    i1 = i1 * (1.0 + cm);
    tcz = tcz + (0.333 + tex_pos);
    var i0 = emi(tcz) * tbw.z;
    tcx = tcx + (0.333 + tex_pos);
    i0 = i0 + emi(tcx) * tbw.x;
    tcy = tcy + (0.333 + tex_pos);
    i0 = i0 + emi(tcy) * tbw.y;
    i0 = i0 * th * th * 1.5;
    a = abs(i0 - 0.5) * 2.0;
    b = abs(1.0 - abs(a - 0.5) * 4.0);
    b = 1.0 - abs(0.5 - b) * 2.0;
    i0 = b * (1.0 - a);
    a = i0 * i1 * 2.0;
    i0 = i0 * (1.0 + cm);
    b = 1.0 - 2.0 * (1.0 - i0) * (1.0 - i1);
    th = 1.0 - abs(va);
    th = th * th;
    th = 0.5 * max(1.0 - th * th * 1.5, 0.0);
    va = 1.1 - abs(va) - b * b * 0.1;
    va = va * va;
    va = va * va * 1.5;
    va = (0.75 + va * 3.0) * max(1.0 - va * va * 1.5, 0.0);
    var c = clamp((0.5 - i1) * 10.0 + 0.5, 0.0, 1.0);
    a = mix(b, a, c);
    a = a * 0.5 + 0.5;
    a = (a * a + a) * 0.5;
    f = 1.0 - f;
    f = f * f;
    ta = (1.0 - ta) - f * f * 6.0;
    let alpha = ta * th * th;
    a = max(a - f * f * 2.5, 0.0);
    b = max(a - 0.5, 0.0);
    b = b * b;
    b = b * b * 4.0;
    a = a * 0.4 * (1.0 - b) + max(1.0 / (1.3 - a) - 1.0, 0.0) * b;
    a = a * (1.0 - fade * fade * 0.5);
    let fcol = vec3(0.45, 0.33, 0.95);
    c = clamp((0.5 - a) * 10.0 + 0.5, 0.0, 1.0);
    var col = vec3(1.0) - (1.0 - fcol) * (1.0 - a) * 2.0;
    col = mix(col, fcol * a * 2.0, vec3(c));
    col = col * 1.5 * (1.0 + f * 3.0);
    col = max(col - 0.1, vec3(0.0));
    col = col * col;
    va = va * va;
    return clamp(vec4(col * va * (0.35 + cm * 4.0), alpha), vec4(0.0), vec4(1.0));
}
}
