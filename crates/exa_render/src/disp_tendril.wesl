// The displacement tendrils (the constant displacement's wisps and a phased
// blow's strand): the `_TDispMat01` record drawn by the engine's surface
// program. Its shading flags carry no lighting, its one layer is the
// emissive `FlameletsA3`, and the strip has a colour stream, so the
// builder's `shading_f` reduces to
//
//     FragColor = vrCol * texture(EmiTex, vrTex) * MatDif.a;
//
// with `vrCol = vCol * 0.003921568` (Bevy's colour attribute, already 0..1)
// and `vrTex` the strip's first coordinate set (the side, 0 or 1, and the
// texture distance along the strand). `MatDif.a` is the record's opacity,
// 0.5. The record's blend word 0x163 (ONE, ONE_MINUS_SRC_ALPHA) is Bevy's
// premultiplied blend; the output is clamped to [0, 1] as the original's
// 8-bit target clamps it. Rules: exa_render::force_effects and
// docs/RENDERING.md (Force effects).

import bevy_pbr::render::forward_io::VertexOutput;

struct DispTendrilParams {
    // `MatDif`: the record's diffuse colour with its opacity in w.
    mat_dif: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> tendril: DispTendrilParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var emi_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var emi_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var tex = vec2(0.0);
    var col = vec4(1.0);
@if(VERTEX_UVS_A) {
    tex = in.uv;
}
@if(VERTEX_COLORS) {
    col = in.color;
}
    let out = col * textureSample(emi_texture, emi_sampler, tex) * tendril.mat_dif.a;
    return clamp(out, vec4(0.0), vec4(1.0));
}
