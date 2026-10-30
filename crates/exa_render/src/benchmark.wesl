// The GPU benchmark's fragment program, the original's own GLSL translated
// one for one: a texture read at the quad's uv, the square roots, the read
// 0.2341 to the right, the dot products, the read 0.1476 down, the lengths,
// the powers, the read 0.3768 left and 0.4276 up, and the sines and cosines.
// The original samples a 512-square texture the hardware repeats, so its
// image carries that addressing.
//
// The vertex stage is the mesh renderer's own quad shader, which hands the
// texture coordinate at location 2 (after the world position and normal).
struct BenchmarkIn {
    @location(2) uv: vec2<f32>,
}

// The original's one projection uniform (set to 2.0), which its program
// declares and never reads. A four-float block keeps the uniform buffer's
// 16-byte stride.
struct BenchmarkParams {
    projection: vec4<f32>,
}

@group(2) @binding(0) var<uniform> params: BenchmarkParams;
@group(2) @binding(1) var texture: texture_2d<f32>;
@group(2) @binding(2) var texture_sampler: sampler;

@fragment
fn fragment(in: BenchmarkIn) -> @location(0) vec4<f32> {
    var t = in.uv;
    var a = textureSample(texture, texture_sampler, in.uv).rgb;
    a.r = sqrt(a.r);
    a.g = sqrt(a.g);
    a.b = sqrt(a.b);
    t.x = t.x + 0.2341;
    var b = textureSample(texture, texture_sampler, t).rgb;
    b.r = dot(a, b);
    b.g = dot(a, a);
    b.b = dot(b, b);
    a = a + b;
    t.y = t.y - 0.1476;
    var c = textureSample(texture, texture_sampler, t).rgb;
    a.r = a.r - length(a);
    a.g = a.g - length(b);
    a.b = a.b - length(c);
    a.r = pow(a.r, 1.2134);
    a.g = pow(a.g, 2.2134);
    a.b = pow(a.b, 3.734);
    t.x = t.x - 0.3768;
    t.y = t.y + 0.4276;
    b = textureSample(texture, texture_sampler, t).rgb;
    b.r = sin(b.r) + cos(b.g);
    b.g = sin(b.g) + cos(b.r);
    b.b = sin(b.b) + cos(b.g);
    return vec4<f32>(a + b, 1.0);
}
