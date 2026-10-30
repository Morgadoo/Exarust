// The force blast's and torrent's ribbons: the original's `effectforce_f`
// (blast, bolt and wave chunks) and, with TORRENT, `effecttorrent_f` (the
// torrent's streams), both under `effectforce_v`. The vertex stage is Bevy's
// mesh stage: `vTex` is the first coordinate set, `vTx2` the second and
// `vCol` the colour bytes, already scaled to 0..1 as the original's
// `vCol · 0.003921568` does. `EmiTex` is the `_ForceNoise` picture and
// `TexOfs` the geometry's offset. The material's blend word is 0x163 (ONE,
// ONE_MINUS_SRC_ALPHA), drawn as Bevy's premultiplied blend; `effectforce_f`
// never writes its alpha, which is taken as 0 (purely additive). The output
// is clamped to [0, 1] as the original's 8-bit target clamps it. Rules:
// exa_render::force_effects and docs/RENDERING.md (Force effects).

import bevy_pbr::render::forward_io::VertexOutput;

struct ForceRibbonParams {
    // `TexOfs`: (0, radius, texture scale) for a blast, (0, scroll, 1) for a
    // torrent; w is unused.
    tex_ofs: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> ribbon: ForceRibbonParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var emi_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var emi_sampler: sampler;

fn emi(tc: vec2<f32>) -> f32 {
    return textureSample(emi_texture, emi_sampler, tc).r;
}

fn emi_lod(tc: vec2<f32>, lod: f32) -> f32 {
    return textureSampleLevel(emi_texture, emi_sampler, tc, lod).r;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var tex = vec2(0.0);
    var tx2 = vec2(0.0);
    var col = vec4(1.0);
@if(VERTEX_UVS_A) {
    tex = in.uv;
}
@if(VERTEX_UVS_B) {
    tx2 = in.uv_b;
}
@if(VERTEX_COLORS) {
    col = in.color;
}
    let tex_ofs = ribbon.tex_ofs.xyz;
    var out = vec4(0.0);
@if(!TORRENT) {
@if(!SMOKE) {
    // effectforce_f
    var tc: vec2<f32>;
    tc.x = tex.x * 0.5;
    tc.y = tex.y * tex.y * 0.125;
    var d = max(tex.y - 0.75, 0.0);
    d = d * d;
    d = 1.0 + d * d * 800.0 * col.b;
    tc.y = tc.y * (d * 0.1 + 0.95);
    tc.y = tc.y * tx2.x + tx2.y * 2.0;
    d = d * (0.1 + tex.y * 0.2) * 1.5;
    tc.x = (tc.x - 0.3) * d + 0.3;
    tc += col.xy;
    tc = tc * tex_ofs.z;
    var a = emi(tc * 0.5);
    tc.x = tc.x + 0.3;
    tc.y = tc.y + 0.3 + tx2.y;
    let lod = (1.0 - col.w) * 4.0;
    var b = emi_lod(tc * 1.25, lod) * 1.5;
    a = a * b * b;
    tc = tc * 2.0;
    d = emi_lod(tc, lod) * 0.5;
    a = a + d * d * (2.0 + tex.y * tex.y);
    a = a * a;
    b = 1.0 - abs(tex.x + tex.x - 1.0);
    b = b * min(10.0 - tex.y * 10.0, 1.0);
    var c = a * b * tex.y;
    b = b * tex.y * (2.0 - tex.y);
    a = (a * a * b * b * 2.0 + b * 0.5) * tex.y * tex.y * tex.y;
    c = c * col.w * (2.0 - col.w);
    b = col.w * col.w;
    a = a * b * b;
    var rgb = vec3(a * 0.3) * col.b;
    rgb = rgb * vec3(0.2, 0.1, 0.8) * (0.5 + tex.y);
    rgb = rgb + vec3(a * 0.03);
    rgb = (rgb * rgb + rgb) * 0.5;
    rgb = rgb + vec3(0.04, 0.0, 0.6) * c * 0.6;
    out = vec4(rgb * 0.25, 0.0);
}
}
@if(TORRENT) {
    // effecttorrent_f (its first `d`, computed and overwritten before any
    // read, is left out).
    var tc = tex * 0.5;
    tc.x = tc.x * (max(tx2.x, 0.0) * 0.5 + 0.5);
    tc = tc * 0.75;
    tc += col.xy;
    tc = tc * tex_ofs.z;
    var a = emi(tc * 0.5);
    tc.y = tc.y + 0.3 + tex_ofs.y;
    var lod = 1.0 - tx2.x;
    lod = lod * lod * 4.0;
    var b = emi_lod(tc * 1.25, lod) * 1.5;
    a = a * b * b;
    tc.y = tc.y + tex_ofs.y;
    tc = tc * 2.0;
    var d = emi_lod(tc, lod) * 0.5;
    a = a + d * d * 2.0;
    a = a * a;
    b = 1.0 - abs(tex.x + tex.x - 1.0);
    var c = b * b * tx2.x + (1.0 - tx2.x);
    b = b * c;
    c = a * b;
    a = a * a * b * b * 2.0 + b * 0.5;
    b = max(tx2.x, 0.0);
    c = c * b * (2.0 - b);
    a = a * b * b;
    d = min(tx2.y, 1.0);
    a = a * d;
    c = c * d * d;
    var rgb = vec3(a * 0.3) * col.b;
    rgb = rgb * vec3(0.2, 0.1, 0.8);
    rgb = rgb + vec3(a * 0.05);
    rgb = (rgb * rgb + rgb) * 0.5;
    rgb = rgb + vec3(0.04, 0.0, 0.6) * c * 0.6;
    out = vec4(rgb * 0.25 * col.a, c * tx2.x * 0.1 * col.a);
}
@if(SMOKE) {
    // effectsmoke_f, the displacement tendril's smoke.
    var tc = vec2(0.0);
    tc.x = tex.x - 0.5;
    tc.x = tc.x * (tx2.x * 0.7 + 0.3) + 0.5;
    tc.x = tc.x * 0.13;
    tc.y = tex.y * 0.38;
    tc += col.xy;
    tc = tc * tex_ofs.z;
    var a = emi(tc * 0.5);
    tc.y = tc.y + 0.3 - tex_ofs.y * 0.25;
    tc.y = tc.y * 1.2;
    var b = emi(tc * 1.25) * 1.5;
    a = a * b * (1.5 - a * a);
    tc.y = tc.y - tex_ofs.y * 0.25;
    tc = tc * 1.75;
    b = emi(tc) * 0.5;
    b = max(b * 3.0 - 1.0, 0.0);
    a = a + b * b * (2.0 - a * a);
    a = a * a;
    b = 1.0 - abs(tex.x + tex.x - 1.0);
    let c = b * b * tx2.x + (1.0 - tx2.x);
    a = a * b * c;
    b = min(tx2.y, 1.0);
    let d = tx2.x * (2.0 - tx2.x);
    a = a * d;
    out = vec4(vec3(0.0), a * col.a * b);
}
    return clamp(out, vec4(0.0), vec4(1.0));
}
