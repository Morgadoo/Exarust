// The displacement shell: the original's `dispshell_v` / `dispshell_f` over
// a copy of a vanishing body's skinned meshes (Vanish, the Long Door). The
// vertex stage is Bevy's skinned mesh stage with the copy's placement added:
// each vertex moves out along its skinned normal by `push` (2 cm, the
// skinned copy's `position + 2·normal`). The fragment stage carries the
// world position and normal into the shell's local frame (`LclMat`, the
// inverse of the body node's frame, composed with the metre-to-centimetre
// scale) and runs `dispshell_f` as shipped: three planar `EmiTex` lookups
// (the `_EffectShell01` record's emissive layer, `FlameletsA3`) rolled by
// `TexOfs`, the fade plane, and the outer rim or, by `Interior`, the
// darkened inside of the silhouette. The colour is black: under the
// record's blend word 0x163 (ONE, ONE_MINUS_SRC_ALPHA, Bevy's premultiplied
// blend) the shell only darkens what lies behind it. The record is
// one-sided (flags 0x480008). The viewer direction is normalised per
// fragment rather than per vertex. The output is clamped to [0, 1] as the
// original's 8-bit target clamps it.
//
// With `DEPTH_ONLY` the same stage draws the body's depth pass: the copy
// unmoved, no colour (the pipeline's colour writes are off) and depth
// written, as the vanish's segment pass draws the hidden body's parts.
// Rules: exa_render::force_effects and docs/RENDERING.md (Force effects).

import bevy_pbr::render::{
    forward_io::{Vertex, VertexOutput, decompress_vertex},
    mesh_functions,
    skinning,
    view_transformations::position_world_to_clip,
};

struct DispShellParams {
    // `LclMat` composed with the metre-to-centimetre scale: world (m) to
    // the shell's local frame (cm).
    local_from_world: mat4x4<f32>,
    // `ViewPos`: the camera in the shell's local frame (cm); w unused.
    view_pos: vec4<f32>,
    // `FdPln`: the fade plane (axis, offset).
    fade_plane: vec4<f32>,
    // `TexPos` in xy, `TexOfs` in z; w unused.
    tex: vec4<f32>,
    // `Fade`: (1 − shown²)².
    fade: f32,
    // `Interior`: 1 − the vanish's shown amount.
    interior: f32,
    // `TexScale`.
    tex_scale: f32,
    // `Opacity`.
    opacity: f32,
    // The copy's push along the skinned normal (m).
    push: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> shell: DispShellParams;
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
    let tex_pos = shell.tex.xy;
    let tex_ofs = shell.tex.z;
    let fade = shell.fade;
    let va = dot(vwr, nrm);
    var tst = va;
    var f = max(1.0 - fade, 0.0) * 3.0;
    var lp = lpos;
    f = clamp(f - dot(lp, shell.fade_plane.xyz) - shell.fade_plane.w, 0.0, 1.0);
    var th = f;
    var a = 0.375 * shell.tex_scale;
    lp.y = lp.y * 0.5 + 0.375;
    var tcz = lp.xy * a + tex_pos;
    var tcy = lp.xz * a + tex_pos;
    var tcx = lp.yz * a + tex_pos;
    tcx.x = tcx.x + sign(nrm.x) * 0.333;
    tcy.y = tcy.y + sign(nrm.y) * 0.333;
    tcz.x = tcz.x - sign(nrm.z) * 0.333;
    var tbw = abs(nrm * nrm);
    tbw = tbw / (tbw.x + tbw.y + tbw.z);
    tcz.y = tcz.y + f * f * 0.02;
    tcx.x = tcx.x + f * f * 0.02;
    tcx.x = tcx.x - tex_ofs * 2.0;
    tcz.y = tcz.y - tex_ofs * 2.0;
    tcy.y = tcy.y + tex_ofs;
    var i1 = emi(tcz) * tbw.z;
    i1 = i1 + emi(tcx) * tbw.x;
    i1 = i1 + emi(tcy) * tbw.y;
    tcz.y = tcz.y - f * f * 0.04;
    tcx.x = tcx.x - f * f * 0.04;
    tcx.x = tcx.x - tex_ofs;
    tcz.y = tcz.y - tex_ofs;
    tcy.y = tcy.y - tex_ofs;
    tcz += 0.333;
    var i0 = emi(tcz) * tbw.z;
    tcx += 0.333;
    i0 = i0 + emi(tcx) * tbw.x;
    tcy += 0.333;
    i0 = i0 + emi(tcy) * tbw.y;
    // The shipped program works out a band of `i0` here and overwrites it
    // unread; only the sum of the two lookups goes on.
    a = (i0 + i1) * 0.7;
    a = a * a;
    let b = a;
    th = 1.0 - abs(va);
    f = th * th;
    a = a - (fade * 2.0) * (1.0 - th + max(fade, 0.0));
    th = 0.4 + th * abs(fade);
    th = th * (a - f) * 0.8;
    th = th * max(1.0 - f * f * 3.0, 0.0);
    tst = max(-tst, 0.0);
    tst = tst * tst * tst;
    a = max(fade, 0.0);
    a = a * (2.0 - a) * (1.0 - a) + a * a * a * a;
    tst = tst * b * (1.0 - a) * 0.6;
    th = mix(th, tst, shell.interior);
    var out = vec4(0.0, 0.0, 0.0, th * shell.opacity);
@if(DEPTH_ONLY) {
    out = vec4(0.0);
}
    return clamp(out, vec4(0.0), vec4(1.0));
}
