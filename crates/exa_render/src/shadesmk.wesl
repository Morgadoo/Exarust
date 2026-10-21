// The original's `shadesmk_f` smoke and mist program: the alpha is the
// surface's facing ratio, the opacity texture read twice at the scrolled
// `UVOfst` (a constant quarter without one) and the vertex colour's alpha;
// the colour is black. The original's fog and dither snippets follow in its
// program; Exarust's surfaces have neither yet. The vertex stage is Bevy's
// mesh stage in place of the original's `shading_v`. Rules: exa_render::smoke
// and docs/RENDERING.md (Smoke and mist).

import bevy_pbr::render::{
    forward_io::VertexOutput,
    mesh_view_bindings as view_bindings,
};

struct SmokeParams {
    // The original's `UVOfst`: (0.5, t + 0.5, t) for the scroll t.
    uv_offset: vec3<f32>,
    // 1 when the record has an opacity texture (the program's `Mat.TexOpc`).
    has_opacity: u32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> smoke: SmokeParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var opacity_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var opacity_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let nrm = normalize(in.world_normal);
    let vwr = normalize(in.world_position.xyz - view_bindings::view.world_position.xyz);
    let ndv = abs(dot(vwr, nrm));
    var a = ndv * 1.5 - 1.0;
    if smoke.has_opacity != 0u {
        var tc = in.uv;
        tc.y = tc.y + smoke.uv_offset.z;
        var t = textureSample(opacity_texture, opacity_sampler, tc).w;
        tc = tc + smoke.uv_offset.xy;
        t = t * textureSample(opacity_texture, opacity_sampler, tc).w;
        a = a + t;
    } else {
        a = a + 0.25;
    }
    // Without a vertex-colour stream the attribute reads GL's default
    // alpha of 1.
    var colour_alpha = 1.0;
@if(VERTEX_COLORS) {
    colour_alpha = in.color.a;
}
    a = (a - 1.0 + colour_alpha) * 0.8;
    return vec4(0.0, 0.0, 0.0, a);
}
