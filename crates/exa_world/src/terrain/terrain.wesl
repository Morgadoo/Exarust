// Terrain layering: up to four palette materials per tile, stacked in order.
// The vertex colour holds each layer's "over" opacity (the first layer is
// opaque); a layer with a blend mask sharpens its opacity against the mask so
// stones and grass blades show through in their own shapes.

import bevy_pbr::render::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
};

@if(PREPASS_PIPELINE)
import bevy_pbr::{
    prepass::io::{VertexOutput, FragmentOutput},
    deferred::functions::deferred_output,
};
@else
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
};

struct TerrainSplat {
    // Linear RGB multiplier per layer; alpha is its cover strength.
    colour: array<vec4<f32>, 4>,
    // Mask sharpness s, t; z is 1 when the layer has a blend mask.
    blend: array<vec4<f32>, 4>,
    // Texture repeats per metre per layer.
    tiling: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> splat: TerrainSplat;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var diffuse0: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var diffuse0_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var diffuse1: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var diffuse1_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(105) var diffuse2: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(106) var diffuse2_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(107) var diffuse3: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(108) var diffuse3_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(109) var mask1: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(110) var mask1_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(111) var mask2: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(112) var mask2_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(113) var mask3: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(114) var mask3_sampler: sampler;

// Stacks layer `k` with colour `c`, opacity `a` and mask value `m` on `below`.
fn stack(below: vec3<f32>, c: vec3<f32>, a: f32, m: f32, k: i32) -> vec3<f32> {
    let b = splat.blend[k];
    if b.z < 0.5 {
        return mix(below, c, a);
    }
    let sharp = clamp((b.x + b.y) * a - b.x * (1.0 - m), 0.0, 1.0);
    return below * clamp(1.0 - sharp * splat.colour[k].a, 0.0, 1.0) + c * sharp;
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    var alpha = vec4<f32>(1.0, 0.0, 0.0, 0.0);
@if(VERTEX_COLORS) {
    alpha = in.color;
}

    let p = in.world_position.xz;
    let uv0 = p * splat.tiling.x;
    let uv1 = p * splat.tiling.y;
    let uv2 = p * splat.tiling.z;
    let uv3 = p * splat.tiling.w;
    var colour = textureSample(diffuse0, diffuse0_sampler, uv0).rgb * splat.colour[0].rgb;
    colour = stack(colour, textureSample(diffuse1, diffuse1_sampler, uv1).rgb * splat.colour[1].rgb,
        alpha.y, textureSample(mask1, mask1_sampler, uv1).r, 1);
    colour = stack(colour, textureSample(diffuse2, diffuse2_sampler, uv2).rgb * splat.colour[2].rgb,
        alpha.z, textureSample(mask2, mask2_sampler, uv2).r, 2);
    colour = stack(colour, textureSample(diffuse3, diffuse3_sampler, uv3).rgb * splat.colour[3].rgb,
        alpha.w, textureSample(mask3, mask3_sampler, uv3).r, 3);
    pbr_input.material.base_color = vec4<f32>(colour, 1.0);

    pbr_input.material.base_color = alpha_discard(
        pbr_input.material.flags,
        pbr_input.material.alpha_cutoff,
        pbr_input.material.base_color
    );

    var out: FragmentOutput;
@if(PREPASS_PIPELINE) {
    out = deferred_output(in, pbr_input);
} @else {
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
}
    return out;
}
