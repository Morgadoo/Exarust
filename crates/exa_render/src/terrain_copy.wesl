// The terrain's composite set copy: the original's built-in full-screen copy
// program, `FragColor.rgb = texture(Tex0, vrTex).rgb`. A block that gives up
// its high-resolution composite for a low set keeps its image by blitting
// each target into the new set instead of re-stacking its layers. Only the
// colour channels are written; the destination targets have no meaningful
// alpha (the colour target's is unused, the normal, gloss and displacement
// targets have none).
//
// `in.uv` is the same 0–1 target coordinate the bake passes write at, so the
// copy preserves orientation. Rules: docs/formats/TERRAIN.md (set copy).

import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;

@group(0) @binding(0) var tex0: texture_2d<f32>;
@group(0) @binding(1) var tex0_sampler: sampler;

@fragment
fn copy_rgb(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(textureSample(tex0, tex0_sampler, in.uv).rgb, 1.0);
}
