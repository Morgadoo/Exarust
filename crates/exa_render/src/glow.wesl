// The thaumaturgy effects' stand-in look until their own programs are
// ported: the vertex colour times the material's tint, added to the frame.
// Rules: docs/THAUMATURGY.md (Drawing).

import bevy_pbr::render::forward_io::VertexOutput;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> tint: vec4<f32>;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var colour = tint;
@if(VERTEX_COLORS) {
    colour = colour * in.color;
}
    return vec4(colour.rgb * colour.a, colour.a);
}
