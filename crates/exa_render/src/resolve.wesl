// The supersampled scene shown at window resolution: bilinear at each
// window pixel's centre is the equal-weight average of its 2 × 2 block, the
// original's resolve. Rules: docs/RENDERING.md (Supersampling).

import bevy_pbr::render::forward_io::VertexOutput;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var scene: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var scene_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(textureSampleLevel(scene, scene_sampler, in.uv, 0.0).rgb, 1.0);
}
