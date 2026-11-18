// The original's post-processing: a bright pass, a separable blur at a
// quarter of the resolution, then the grading curve and the bloom combine.
// Rules: docs/RENDERING.md (Post-processing).

import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;

@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
// The final pass also reads the blurred bloom and the settings.
@group(0) @binding(2) var bloom: texture_2d<f32>;
@group(0) @binding(3) var<uniform> settings: PostSettings;

struct PostSettings {
    // Bloom strength (1 in play), desaturation (0 in play), the sharpen
    // supersampling turns on, unused.
    bloom_strength: f32,
    desaturate: f32,
    sharpen: f32,
    // The blur's last tap index (taps - 1).
    last: i32,
    // The original's `Blur`: the screen fade's weight, the bloom's scale
    // (the strength, faded in with the screen), the desaturation mask (how
    // far the scene gives way to its bloom) and the bloom term.
    blur: vec4<f32>,
    // The blur kernel (`Filter`), four weights a slot: the centre, then
    // each symmetric pair's weight (`exa_render::post::bloom_kernel`).
    filter: array<vec4<f32>, 8>,
}

// The scene is stored in 8 bits by the original, so it clamps at 1.
fn scene(uv: vec2<f32>) -> vec3<f32> {
    return clamp(textureSampleLevel(source, source_sampler, uv, 0.0).rgb, vec3(0.0), vec3(1.0));
}

@fragment
fn bright(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let c = scene(in.uv);
    return vec4(clamp((c + c * c) * 0.8, vec3(0.0), vec3(1.0)), 1.0);
}

fn weight(i: i32) -> f32 {
    return settings.filter[i / 4][i % 4];
}

// The view's kernel: the centre weight, then a symmetric pair for each
// further tap, two half-resolution texels apart.
fn blur(uv: vec2<f32>, step: vec2<f32>) -> vec4<f32> {
    var sum = textureSampleLevel(source, source_sampler, uv, 0.0).rgb * weight(0);
    for (var i = 1; i <= settings.last; i++) {
        let offset = step * f32(2 * i);
        sum += textureSampleLevel(source, source_sampler, uv + offset, 0.0).rgb * weight(i);
        sum += textureSampleLevel(source, source_sampler, uv - offset, 0.0).rgb * weight(i);
    }
    return vec4(sum, 1.0);
}

@fragment
fn blur_x(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // The source is the half-resolution bright pass.
    let size = vec2<f32>(textureDimensions(source));
    return blur(in.uv, vec2(1.0 / size.x, 0.0));
}

@fragment
fn blur_y(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // Offsets stay in half-resolution texels on the quarter-resolution source.
    let size = vec2<f32>(textureDimensions(source)) * 2.0;
    return blur(in.uv, vec2(0.0, 1.0 / size.y));
}

@fragment
fn finish(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let full = textureSampleLevel(source, source_sampler, in.uv, 0.0);
    var c = clamp(full.rgb, vec3(0.0), vec3(1.0));
    // The sharpen: four diagonal samples half a pixel out (each a 2 × 2
    // average); flat areas are unsharp-masked, strong edges eased toward
    // their neighbours.
    if settings.sharpen > 0.01 {
        let px = 0.5 / vec2<f32>(textureDimensions(source));
        var around = vec3(0.0);
        var contrast = 0.0;
        for (var i = 0; i < 4; i++) {
            let sx = select(-1.0, 1.0, i == 1 || i == 2);
            let sy = select(-1.0, 1.0, i >= 2);
            let s = scene(in.uv + px * vec2(sx, sy));
            around += s;
            let diff = c - s;
            contrast += dot(diff, diff);
        }
        let mean = (around - c) * 0.33333;
        contrast *= 16.0;
        let w = (min(contrast * 0.16, 0.8) - pow(clamp(1.0 - contrast, 0.0, 1.0), 16.0)) * settings.sharpen;
        c = c * (1.0 - w) + mean * w;
    }
    let d = settings.desaturate;
    c = mix(c, vec3(c.g), d);
    c *= c.g * 2.0 * d + 1.0 - d * 0.5;
    // The grading curve: a strong lift of the mid-tones, 1 stays 1.
    let b = 0.9 * c * c + 0.1 * c;
    c = 7.05 * b / (6.5 * b + 0.55);
    // Bloom, combined like a screen; the fades scale the scene's share (a
    // weight of 0 is black), lift the bloom and swap the scene for it.
    let fade = settings.blur;
    let blm = max(textureSampleLevel(bloom, source_sampler, in.uv, 0.0).rgb - vec3(0.03), vec3(0.0));
    c = mix(c, blm, fade.z);
    let glow = blm * 1.75 * fade.y + vec3(fade.w);
    let out = vec3(1.0) - clamp((vec3(1.0) - c * fade.x) * (vec3(1.0) - glow * glow), vec3(0.0), vec3(1.0));
    return vec4(out, full.a);
}
