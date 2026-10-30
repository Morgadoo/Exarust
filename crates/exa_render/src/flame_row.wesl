// The flame row's fragment stage: the original's `_FlameMat01`. `vrTex` is
// the ribbon's texture coordinate, `vrCol` the vertex colour scaled to 0..1;
// `Anim` rolls the noise, `EmiTex` is the object material's texture and
// `GrdTex` the five-texel palette. The stage writes only the colour; the
// palette's ramp, clamped, is the result. Rules:
// exa_render::flame_row and docs/RENDERING.md (Fires).

import bevy_pbr::render::forward_io::VertexOutput;

struct FlameRowAnim {
    // `Anim`: (texture offset, twice its negative, half of one less the
    // fade); w is unused.
    anim: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> frame: FlameRowAnim;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var emi_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var emi_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var grd_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var grd_sampler: sampler;

fn emi(tc: vec2<f32>) -> f32 {
    return textureSample(emi_texture, emi_sampler, tc).r;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var col = vec4(1.0);
@if(VERTEX_COLORS) {
    col = in.color;
}
    let vr = in.uv;
    let anim = frame.anim.xyz;

    // The face-on sample: the row's width across, the noise rolled along.
    var r0 = vr;
    r0.y = r0.y * 0.25;
    r0.x = r0.x * 0.5 + anim.x;
    r0.y = (r0.y + anim.y * 2.0) * 0.25;
    var tex = emi(r0);

    // The row's vertical falloff and its complement.
    var a = clamp((1.0 - vr.y * 0.3) * 1.5, 0.0, 1.0);
    var b = min(vr.y * 3.0, 1.0);
    b = 1.0 - b * a;
    b = b * b;
    let d = b;
    let c = 1.0 - b;
    tex = tex + c * 1.5;

    // A second, tighter scroll.
    r0 = vr;
    r0.x = r0.x - anim.x;
    r0.y = r0.y * 0.0625 + anim.y;
    a = emi(r0);
    tex = tex * a;

    r0 = vr;
    r0.y = r0.y * 0.0625 + anim.y;
    b = emi(r0);
    b = b * b;
    b = b + c * c * 1.5;
    tex = mix(b, tex, vr.y);

    // A third scroll, twice as fast.
    r0 = vr;
    r0.y = r0.y * 0.0625 + anim.y * 3.0;
    r0 = r0 * 2.0;
    a = emi(r0);
    a = a - vr.y;
    a = mix(1.0, a, vr.y);
    tex = tex * a * 0.45;

    // The vertex colour's red squares off the rim, then the fade and the
    // palette lookup.
    a = col.x * col.x;
    tex = tex - anim.z;
    tex = tex - a;
    var r = vec2<f32>(0.0);
    r.x = clamp(tex - d, 0.0, 1.0);
    r.y = r.x;
    let rgb = textureSample(grd_texture, grd_sampler, r).rgb;
    return vec4(rgb, 1.0);
}
