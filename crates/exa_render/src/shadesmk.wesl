// Smoke and mist, as the original's `shadesmk_f` draws them: the texture
// read at the animated `UVOfst`, tinted and alpha-blended. The vertex stage
// is the shared one (`shading_v` in the original; Bevy's mesh vertex here).
// Rules: exa_render::smoke and docs/RENDERING.md.

import bevy_pbr::render::forward_io::VertexOutput;

struct SmokeParams {
    uv_offset: vec2<f32>,
    tint: vec4<f32>,
    state_key: u32,
    pad: u32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> smoke: SmokeParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var smoke_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var smoke_sampler: sampler;

// The original's texture LOD bias on colour textures (exa.wesl).
const TEXTURE_BIAS: f32 = -0.75;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSampleBias(smoke_texture, smoke_sampler, in.uv + smoke.uv_offset, TEXTURE_BIAS);
    return vec4(texel.rgb * smoke.tint.rgb, texel.a * smoke.tint.a);
}
