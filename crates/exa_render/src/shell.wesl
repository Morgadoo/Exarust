// The rooms' shells and fade overlays: a flat colour, unlit (the original's
// flat program, FragColor = Colour). Opaque black for shells; black at the
// fade's alpha for overlays. Rules: docs/RENDERING.md (Rooms and black fill).

import bevy_pbr::render::forward_io::VertexOutput;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> colour: vec4<f32>;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return colour;
}
