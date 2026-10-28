// The terrain's layer stacking, shared by the terrain draw (`terrain.wesl`
// in exa_world, which stacks the layers per pixel) and the composite bake
// (`terrain_bake.wesl`, which writes the same stack into a block's set).
// Each function takes the layer's values explicitly so both callers read
// them from their own uniform. Rules: docs/RENDERING.md (Terrain).

// Opacity of a layer from its over opacity `a` and mask value `m`, with
// sharpness `s`, `t` (an unmasked layer keeps `a`).
fn sharpen(a: f32, m: f32, s: f32, t: f32, masked: bool) -> f32 {
    if !masked {
        return a;
    }
    return clamp((s + t) * a - s * (1.0 - m), 0.0, 1.0);
}

// A colour (or gloss) `c` of over opacity `a` and mask value `m` stacked on
// `below`. `blend` is the layer's mask sharpness s, t and its masked flag
// (z > ½); `strength` its cover strength (the colour's alpha).
fn stack_colour(below: vec3<f32>, c: vec3<f32>, a: f32, m: f32, blend: vec4<f32>, strength: f32) -> vec3<f32> {
    if blend.z < 0.5 {
        return mix(below, c, a);
    }
    let sharp = sharpen(a, m, blend.x, blend.y, true);
    return below * clamp(1.0 - sharp * strength, 0.0, 1.0) + c * sharp;
}

// A normal xy `n` stacked on `below` with the normal pass's sharpness
// (`normal.xy`).
fn stack_xy(below: vec2<f32>, n: vec2<f32>, a: f32, m: f32, normal: vec4<f32>, masked: bool) -> vec2<f32> {
    return mix(below, n, sharpen(a, m, normal.x, normal.y, masked));
}

// Tangent-space normal xy from a normal-map sample and the layer's
// normal-map strength.
fn normal_xy(sample: vec4<f32>, strength: f32) -> vec2<f32> {
    return (sample.xy * 2.0 - 1.0) * strength;
}

// A layer's parallax height from its height texel `b`: f20 + (B − f21)·f22,
// with `q.yzw` the layer's base, reference and scale.
fn layer_height(b: f32, q: vec4<f32>) -> f32 {
    return q.y + (b - q.z) * q.w;
}
