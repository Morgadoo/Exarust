// The force-shell aura: the original's `effectshell_v` / `effectshell_f`
// over a copy of the caster's skinned meshes (or a held body's or weapon's
// rigid ones), and with `STRIKES` the strikes program `effectfrcstrk_f`
// (fixed 0.01 coordinates, four ripple points, its own rim and the
// `Gradient` dimming), and with `SCAN` the teleporter scan's
// `effectscan_f` (fixed 0.03 coordinates, no ripple points, the fade term
// clamped to 100 instead of 1, and the `Colour` uniform for the violet
// ramp, its alpha limit and no `ColourInt`). The vertex stage is Bevy's
// skinned mesh stage with the copy's placement added: each vertex moves out
// along its skinned normal by `push` (2 cm, the skinned copy's
// `position + 2·normal`). The fragment stage carries the world position and
// normal into the shell's local frame (`LclMat`, the inverse of the node's
// frame for a skinned copy, here composed with the metre-to-centimetre
// scale) and runs `effectshell_f` as shipped: three planar `EmiTex` lookups
// blended by the normal, the eight ripple points, the fade plane, the rim
// and the violet colour ramp. The viewer direction is normalised per
// fragment rather than per vertex. The `_ForceAura01` blend word is 0x163
// (ONE, ONE_MINUS_SRC_ALPHA), Bevy's premultiplied blend; the record is
// one-sided (flags 0x480008). The output is clamped to [0, 1] as the
// original's 8-bit target clamps it. Rules: exa_render::force_effects and
// docs/RENDERING.md (Force effects).

import bevy_pbr::render::{
    forward_io::{Vertex, VertexOutput, decompress_vertex},
    mesh_functions,
    skinning,
    view_transformations::position_world_to_clip,
};

struct ForceShellParams {
    // `LclMat` composed with the metre-to-centimetre scale: world (m) to
    // the shell's local frame (cm).
    local_from_world: mat4x4<f32>,
    // `ViewPos`: the camera in the shell's local frame (cm); w unused.
    view_pos: vec4<f32>,
    // `FdPln`: the fade plane (axis, offset).
    fade_plane: vec4<f32>,
    // `ColPt`: the eight ripple points, their shown amount in w.
    colour_points: array<vec4<f32>, 8>,
    // `TexPos` in xy; zw unused (`TexOfs` has no reader in the program).
    tex_pos: vec4<f32>,
    // `ColourInt`.
    colour_int: f32,
    // `TexScale`.
    tex_scale: f32,
    // `Fade`: 1 − shown.
    fade: f32,
    // The copy's push along the skinned normal (m).
    push: f32,
    // `Gradient` (the strikes program): the brightest point (cm, local)
    // and the dimming per centimetre from it.
    gradient: vec4<f32>,
    // `Colour` (the scan program): its colour scaled by the scan's colour
    // scale.
    colour: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> shell: ForceShellParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var emi_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var emi_sampler: sampler;

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let v = decompress_vertex(vertex_in, vertex_in.instance_index);
    var world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
@if(SKINNED) {
    world_from_local = skinning::skin_model(v.joint_indices, v.joint_weights, vertex_in.instance_index);
}
    var normal = vec3(0.0, 1.0, 0.0);
@if(VERTEX_NORMALS && SKINNED) {
    normal = skinning::skin_normals(world_from_local, v.normal);
}
@if(VERTEX_NORMALS && !SKINNED) {
    normal = mesh_functions::mesh_normal_local_to_world(v.normal, vertex_in.instance_index);
}
    var world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(v.position, 1.0));
    world = vec4(world.xyz + normal * shell.push, 1.0);
    out.position = position_world_to_clip(world.xyz);
    out.world_position = world;
    out.world_normal = normal;
@if(VERTEX_OUTPUT_INSTANCE_INDEX) {
    out.instance_index = vertex_in.instance_index;
}
    return out;
}

fn emi(tc: vec2<f32>) -> f32 {
    return textureSample(emi_texture, emi_sampler, tc).r;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let lpos = (shell.local_from_world * vec4(in.world_position.xyz, 1.0)).xyz;
    let nrm = normalize((shell.local_from_world * vec4(in.world_normal, 0.0)).xyz);
    let vwr = normalize(lpos - shell.view_pos.xyz);
    let tex_pos = shell.tex_pos.xy;
    let fade = shell.fade;
    let colour_int = shell.colour_int;
    var va = dot(vwr, nrm);
    var a: f32;
    var f = (1.0 - fade) * 3.0;
    var f_max = 1.0;
@if(SCAN) {
    // `effectscan_f` lets the fade term run to 100.
    f_max = 100.0;
}
    f = clamp(f - dot(lpos, shell.fade_plane.xyz) - shell.fade_plane.w, 0.0, f_max);
    var th = f;
    a = 0.5 * shell.tex_scale;
@if(STRIKES) {
    // `effectfrcstrk_f` scales its coordinates by a fixed 0.01.
    a = 0.01;
}
@if(SCAN) {
    // `effectscan_f` scales its coordinates by a fixed 0.03.
    a = 0.03;
}
    var tcz = lpos.xy * a;
    var tcy = lpos.xz * a;
    var tcx = lpos.yz * a;
    tcx.x = tcx.x + sign(nrm.x) * 0.333;
    tcy.y = tcy.y + sign(nrm.y) * 0.333;
    tcz.x = tcz.x - sign(nrm.z) * 0.333;
    var tbw = abs(nrm * nrm);
    tbw = tbw / (tbw.x + tbw.y + tbw.z);
    tcz.y = tcz.y + f * f * 0.2;
    tcx.x = tcx.x + f * f * 0.2;
    var to = tcz * 0.75 + tex_pos;
    var i1 = emi(to) * tbw.z;
    to = tcx * 0.75 + tex_pos;
    i1 = i1 + emi(to) * tbw.x;
    to = tcy * 0.75 + tex_pos;
    i1 = i1 + emi(to) * tbw.y;
    tcz.y = tcz.y - f * f * 0.4;
    tcx.x = tcx.x - f * f * 0.4;
    var cm = 0.0;
    var cf = 0.0;
    // The shell takes eight ripple points; the strikes program four, each
    // brightening half as much.
    var points = 8;
    var bright = 0.15;
@if(STRIKES) {
    points = 4;
    bright = 0.07;
}
@if(SCAN) {
    // `effectscan_f` has no ripple points.
    points = 0;
}
    for (var i = 0; i < points; i++) {
        let point = shell.colour_points[i];
        let cp = lpos - point.xyz;
        a = dot(cp, cp);
        a = max(1.0 - a * 0.0005, 0.0);
        cf = cf + a * a * point.w * bright;
        cm = cm + a * point.w * 0.03;
    }
    cm = cm * abs(va);
    i1 = i1 + min(cf, 1.8);
    let ta_base = i1;
    i1 = i1 + max(2.0 - f * 4.0, 0.0);
    i1 = max(i1 - f * f * 0.25, 0.0);
    i1 = (i1 * (2.0 - i1) + i1) * 0.5;
    a = abs(i1 - 0.5) * 2.0;
    var b = abs(1.0 - abs(a - 0.5) * 4.0);
    b = 1.0 - abs(0.5 - b) * (0.5 + i1);
    b = b * (1.0 - a);
    i1 = (b * b + b) * 0.55;
    i1 = i1 * (1.0 + cm);
    tcz += 0.333 + tex_pos;
    var i0 = emi(tcz) * tbw.z;
    tcx += 0.333 + tex_pos;
    i0 = i0 + emi(tcx) * tbw.x;
    tcy += 0.333 + tex_pos;
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
    f = max(1.0 - f, 0.0);
    var c = f + cf;
@if(!STRIKES) {
    va = 1.1 - max(-va, 0.0) + b * b * 0.1;
    va = (va + c) * (1.0 + fade) * max(1.0 - va * va * 1.0, 0.0);
}
@if(!STRIKES && !SCAN) {
    va = va * colour_int;
}
@if(STRIKES) {
    c = f;
    var t = max(-va, 0.0);
    va = t - 0.1 + b * b * 0.2;
    t = t * t;
    va = (va * va + c) * (1.0 + fade) * max(1.0 - t * t * 1.0, 0.0);
    va = va * colour_int * 2.0;
}
    c = clamp((0.5 - i1) * 10.0 + 0.5, 0.0, 1.0);
    a = mix(b, a, c);
    a = a * 0.5 + 0.5;
    a = (a * a + a) * 0.5;
    var alpha_scale = min(colour_int, 1.0);
@if(SCAN) {
    alpha_scale = min(shell.colour.a, 1.0);
}
@if(STRIKES) {
    // Brightest at the gradient's point, dimming away from it; no colour
    // limit on the alpha.
    let dst = 1.0 - max(length(lpos - shell.gradient.xyz) * shell.gradient.w, 0.0);
    a = a * dst;
    alpha_scale = 1.0;
}
    f = f * f;
    let ta = (1.0 - ta_base) - f * f * 6.0;
    let alpha = ta * th * th * 0.5 * alpha_scale;
    a = max(a - f * f * 2.5, 0.0);
    b = max(a - 0.5, 0.0);
    b = b * b;
    b = b * b * 4.0;
    a = a * 0.4 * (1.0 - b) + max(1.0 / (1.3 - a) - 1.0, 0.0) * b;
    a = a * (1.0 - fade * fade * 0.5);
    var fcol = vec3(0.45, 0.33, 0.95);
    // The ramp's brightening with the fade term: `3 − ColourInt` in the
    // shell, a plain 3 in the scan.
    var lift = 3.0 - colour_int;
@if(SCAN) {
    fcol = shell.colour.rgb * 0.3;
    lift = 3.0;
}
    c = clamp((0.5 - a) * 10.0 + 0.5, 0.0, 1.0);
    var col = vec3(1.0) - (1.0 - fcol) * (1.0 - a) * 2.0;
    col = mix(col, fcol * a * 2.0, vec3(c));
    col = col * 1.5 * (1.0 + f * lift);
    col = max(col - 0.1, vec3(0.0));
    col = col * col;
    va = va * va;
    let rgb = col * va * (0.35 + cm * 4.0);
    return clamp(vec4(rgb, alpha), vec4(0.0), vec4(1.0));
}
