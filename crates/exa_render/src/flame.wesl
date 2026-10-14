// Fire particles: the particle texture times the particle's colour (linear,
// alpha the key's opacity), written premultiplied: colour is added in full
// and the background dimmed by the alpha. Rules: docs/RENDERING.md (Fires).

import bevy_pbr::render::forward_io::VertexOutput;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var particle: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var particle_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = textureSample(particle, particle_sampler, in.uv);
    var c = vec4(1.0);
@if(VERTEX_COLORS) {
    c = in.color;
}
    return vec4(t.rgb * c.rgb, t.a * c.a);
}
