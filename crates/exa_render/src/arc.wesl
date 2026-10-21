// Electricity arcs: the original's `_ElecMat01` fragment program, unlit and
// additive. The emissive texture's red channel is mixed a third of the way
// into the ribbon's V coordinate, folded to a ridge, and raised to 24 and 6
// for the electric core and its soft edge. The vertex colour is mixed toward
// EleCol by the core and the edge colour added, then the result squared.
// Rules: docs/RENDERING.md (Electricity).

import bevy_pbr::render::forward_io::VertexOutput;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var arc_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var arc_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<uniform> ele_colour: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var<uniform> edg_colour: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var<uniform> tex_offset: vec4<f32>;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let tc = (in.uv + tex_offset.xy) * 0.25;
    var a = textureSample(arc_texture, arc_sampler, tc).r;
    a = mix(in.uv.y, a, 0.3);
    a = 1.0 - abs(a + a - 1.0);
    let b = pow(a, 24.0);
    a = pow(a, 6.0);
    var c = vec4(1.0);
@if(VERTEX_COLORS) {
    c = in.color;
}
    var col = mix(c.rgb, ele_colour.rgb, b);
    col = col * b;
    col = col + edg_colour.rgb * a * 2.0;
    col = col * col;
    return vec4(col * c.a, 1.0);
}
