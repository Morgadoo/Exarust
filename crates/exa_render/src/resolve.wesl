// The supersampled scene shown at window resolution: bilinear at each
// window pixel's centre is the equal-weight average of its 2 × 2 block, the
// original's resolve of its four sub-frames. `subframe` 0–3 (the debug view,
// `EXA_SUBFRAME`) shows that pixel of each block alone. Rules:
// docs/RENDERING.md (Supersampling).

import bevy_pbr::render::forward_io::VertexOutput;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var scene: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var scene_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<uniform> subframe: i32;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    if subframe >= 0 {
        let size = vec2<i32>(textureDimensions(scene));
        let block = vec2<i32>(floor(in.uv * vec2<f32>(size / 2)));
        let at = min(block * 2 + vec2(subframe & 1, subframe >> 1), size - 1);
        return vec4(textureLoad(scene, at, 0).rgb, 1.0);
    }
    return vec4(textureSampleLevel(scene, scene_sampler, in.uv, 0.0).rgb, 1.0);
}
