// Blood droplets, as the original's `_BloodPrt02` draws them: the droplet
// texture mixed toward white by the droplet's alpha (its vertex colour),
// written opaque and multiplied into the frame. Rules: docs/RENDERING.md
// (Blood sprays).

import bevy_pbr::render::forward_io::VertexOutput;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var droplet: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var droplet_sampler: sampler;

// The original's texture LOD bias on colour textures (exa.wesl).
const TEXTURE_BIAS: f32 = -0.75;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSampleBias(droplet, droplet_sampler, in.uv, TEXTURE_BIAS).rgb;
    var alpha = 1.0;
@if(VERTEX_COLORS) {
    alpha = in.color.a;
}
    return vec4(mix(vec3(1.0), texel, alpha), 1.0);
}
