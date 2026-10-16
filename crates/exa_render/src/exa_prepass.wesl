// Depth-only passes (shadow maps) for the original's materials: alpha-cut
// surfaces (cobwebs, grates, foliage) cut their shadows the way they cut
// their colour, with the same alpha rule as exa.wesl. Rules:
// docs/RENDERING.md (Lights).

import bevy_pbr::prepass::io::{VertexOutput, FragmentOutput};

struct ExaParams {
    diffuse: vec4<f32>,
    specular: vec4<f32>,
    emissive: vec4<f32>,
    flags: u32,
    pad0: u32,
    pad1: u32,
    pad2: u32,
    gi_origin: vec4<f32>,
    gi_extent: vec4<f32>,
    displacement: vec4<f32>,
    subsurface: vec4<f32>,
}

const DIFFUSE_MAP: u32 = 1u;
const OPACITY_MAP: u32 = 16u;
// The alpha cut of masked materials.
const CUTOFF: f32 = 0.5;

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> material: ExaParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var diffuse_map: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var diffuse_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(109) var opacity_map: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(110) var opacity_sampler: sampler;

fn has(bit: u32) -> bool {
    return (material.flags & bit) != 0u;
}

fn cut(in: VertexOutput) {
    var uv = vec2(0.0);
@if(VERTEX_UVS_A) {
    uv = in.uv;
}
    var alpha = material.diffuse.a;
    if has(OPACITY_MAP) {
        let o = textureSample(opacity_map, opacity_sampler, uv).a;
        alpha *= o * o;
    } else if has(DIFFUSE_MAP) {
        alpha *= textureSample(diffuse_map, diffuse_sampler, uv).a;
    }
    if alpha < CUTOFF {
        discard;
    }
}

@if(PREPASS_FRAGMENT)
@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    cut(in);
    var out: FragmentOutput;
@if(NORMAL_PREPASS)
    out.normal = vec4(in.world_normal * 0.5 + vec3(0.5), 1.0);
    return out;
}

@else
@fragment
fn fragment(in: VertexOutput) {
    cut(in);
}
