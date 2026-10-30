// The Mind sense overlay's sphere, as the original's sight program runs it.
// The vertex stage is the original's position/normal/texture program: the
// sphere's local position dotted with `TexVecU` and `TexVecV` gives the
// noise coordinates and its normal dotted with `EyeVec`, squared, gives the
// facing. The fragment stage cuts the noise picture's red channel by
// `MindCol1.w` and `MindCol2.w` to mix the three colours, weights the result
// by `clamp((1 − facing)·−0.3 + tex, 0, 1)`, squares and doubles it and
// writes `MindCol0.w · facing` as alpha. Bevy's mesh stage supplies the
// model and view-projection matrices the original's `ObjMat` holds.
// Rules: exa_render::mind_sight and docs/RENDERING.md (Mind sight).

import bevy_pbr::render::{
    forward_io::{Vertex, decompress_vertex},
    mesh_functions,
    view_transformations::position_world_to_clip,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> tex_u: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> tex_v: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<uniform> eye: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var<uniform> mind_col0: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var<uniform> mind_col1: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(5) var<uniform> mind_col2: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(6) var emi_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(7) var emi_sampler: sampler;

struct SightOut {
    @builtin(position) position: vec4<f32>,
    @location(0) vr_tex: vec3<f32>,
}

@vertex
fn vertex(vertex_in: Vertex) -> SightOut {
    let v = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
    let world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(v.position, 1.0));
    var out: SightOut;
    out.position = position_world_to_clip(world.xyz);
    var normal = vec3(0.0, 1.0, 0.0);
@if(VERTEX_NORMALS) {
    normal = v.normal;
}
    let a = max(dot(normal, eye.xyz), 0.0);
    out.vr_tex = vec3(dot(v.position, tex_u.xyz), dot(v.position, tex_v.xyz), a * a);
    return out;
}

@fragment
fn fragment(in: SightOut) -> @location(0) vec4<f32> {
    let tex = textureSample(emi_texture, emi_sampler, in.vr_tex.xy).r;
    var a = clamp((tex - mind_col1.w) * 8.0, 0.0, 1.0);
    var col = mix(mind_col0.xyz, mind_col1.xyz, a);
    a = clamp((tex - mind_col2.w) * 8.0, 0.0, 1.0);
    col = mix(col, mind_col2.xyz, a);
    let b = clamp(1.0 - in.vr_tex.z, 0.0, 1.0);
    a = clamp(b * -0.3 + tex, 0.0, 1.0);
    col = clamp(col * a, vec3(0.0), vec3(1.0));
    return vec4(col * col * 4.0, mind_col0.w * in.vr_tex.z);
}
