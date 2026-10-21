// The pixel-space 2-D batcher's draw: a texture (white when untextured)
// times the material colour times the vertex colour, alpha-blended. The
// vertices arrive in clip space from the batcher. Rules: exa_render::quads.

import bevy_pbr::render::forward_io::VertexOutput;

struct QuadParams {
    colour: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> quad: QuadParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var quad_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var quad_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var colour = quad.colour;
@if(VERTEX_COLORS) {
    colour = colour * in.color;
}
    let texel = textureSample(quad_texture, quad_sampler, in.uv);
    return vec4(texel.rgb * colour.rgb, texel.a * colour.a);
}
