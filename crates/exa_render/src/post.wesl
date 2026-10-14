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
    pad1: f32,
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

// Centre weight, then 15 symmetric pairs two half-resolution texels apart.
const WEIGHTS = array<f32, 16>(
    0.0711, 0.0662, 0.0615, 0.0570, 0.0526, 0.0485, 0.0445, 0.0406,
    0.0370, 0.0335, 0.0302, 0.0270, 0.0240, 0.0213, 0.0186, 0.0162,
);

fn blur(uv: vec2<f32>, step: vec2<f32>) -> vec4<f32> {
    var sum = textureSampleLevel(source, source_sampler, uv, 0.0).rgb * WEIGHTS[0];
    for (var i = 1; i < 16; i++) {
        let offset = step * f32(2 * i);
        sum += textureSampleLevel(source, source_sampler, uv + offset, 0.0).rgb * WEIGHTS[i];
        sum += textureSampleLevel(source, source_sampler, uv - offset, 0.0).rgb * WEIGHTS[i];
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
    // Bloom, combined like a screen.
    let blurred = textureSampleLevel(bloom, source_sampler, in.uv, 0.0).rgb;
    let glow = max(blurred - vec3(0.03), vec3(0.0)) * 1.75 * settings.bloom_strength;
    let out = vec3(1.0) - clamp((vec3(1.0) - c) * (vec3(1.0) - glow * glow), vec3(0.0), vec3(1.0));
    return vec4(out, full.a);
}
