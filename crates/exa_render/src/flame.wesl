// Fire particles: the particle texture times the particle's colour (linear,
// alpha the key's opacity), written premultiplied: colour is added in full
// and the background dimmed by the alpha. With ADDITIVE (the level portals'
// `_PrtlPrt01`, blend word 0x101: ONE, ONE) the alpha is written as zero,
// so the colour adds in full; its picture (`PrtDot02`) is alpha-only, the
// shape in its alpha over white, so the colour is weighted by the
// texture's alpha [guess: the material program's handling of an
// alpha-only picture is not read]. Rules: docs/RENDERING.md (Fires, Level
// portals).

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
    var colour = t.rgb * c.rgb;
    var alpha = t.a * c.a;
@if(ADDITIVE) {
    colour = colour * t.a;
    alpha = 0.0;
}
    return vec4(colour, alpha);
}
