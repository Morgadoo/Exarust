// The Force Barricade's wall: the original's `effectfbrcd_v` with
// `effectfbrcd_f`. The mesh is the wall's slab in centimetres (its two
// halves slid toward the middle by the frame, never scaled: the program
// shapes the wall's height and ends itself), and the entity's transform
// carries the node's frame with the centimetre-to-metre scale, so `vPos` is
// the original's attribute. The vertex stage places it and hands on the
// point itself (`vrPos`), its texture coordinates (`vrTex`) and the unit
// vector from `ViewPos` (the camera in the node's frame, cm) to it
// (`vrVwr`). The fragment stage is the program as shipped: `BExt` (the
// node's half extents), `TexPos` (the wall's texture place), `TexOfs` (the
// super-sampling pass's offset, 0 with Exarust's one draw a frame), the eight
// ripple points of `ColPt` (their shown amount in z), `Fade`, and `EmiTex`
// (the `_ForceNoise` picture) looked up twice. The `_ForceBarrier01`
// record's blend word is 0x163 (ONE, ONE_MINUS_SRC_ALPHA), Bevy's
// premultiplied blend. The output is clamped to [0, 1] as the original's
// 8-bit target clamps it.
// Rules: exa_render::force_effects and docs/RENDERING.md (Force effects).

import bevy_pbr::render::{
    mesh_functions,
    view_transformations::position_world_to_clip,
};

struct BarricadeParams {
    // `ViewPos`: the camera in the node's frame (cm); w unused.
    view_pos: vec4<f32>,
    // `BExt`: the node's half extents (cm); w unused.
    extent: vec4<f32>,
    // `TexPos` in xy, `TexOfs` in zw.
    tex_pos: vec4<f32>,
    // `ColPt`: the eight ripple points (cm, the node's frame), their shown
    // amount in z; w unused.
    colour_points: array<vec4<f32>, 8>,
    // `Fade`.
    fade: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> wall: BarricadeParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var emi_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var emi_sampler: sampler;

struct WallVertex {
    @builtin(instance_index) instance_index: u32,
    // `vPos` (cm).
    @location(0) position: vec3<f32>,
    // `vTex`.
    @location(2) uv: vec2<f32>,
}

struct WallVaryings {
    @builtin(position) position: vec4<f32>,
    @location(0) vr_tex: vec2<f32>,
    @location(1) vr_vwr: vec3<f32>,
    @location(2) vr_pos: vec3<f32>,
}

@vertex
fn vertex(vertex_in: WallVertex) -> WallVaryings {
    var out: WallVaryings;
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
    let world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(vertex_in.position, 1.0));
    out.position = position_world_to_clip(world.xyz);
    out.vr_tex = vertex_in.uv;
    out.vr_pos = vertex_in.position;
    out.vr_vwr = normalize(vertex_in.position - wall.view_pos.xyz);
    return out;
}

fn emi(tc: vec2<f32>) -> f32 {
    return textureSample(emi_texture, emi_sampler, tc).r;
}

@fragment
fn fragment(in: WallVaryings) -> @location(0) vec4<f32> {
    let b_ext = wall.extent.xyz;
    let tex_pos = wall.tex_pos.xy;
    let tex_ofs = wall.tex_pos.zw;
    let fade = wall.fade;
    let vr_pos = in.vr_pos;
    let vr_vwr = in.vr_vwr;
    let tr = abs((b_ext - vr_pos * sign(vr_vwr)) / vr_vwr);
    var th = min(min(tr.x, tr.y), tr.z) * 0.1;
    var otc: vec2<f32>;
    otc.x = (vr_pos.x + th * vr_vwr.x * 5.0) / b_ext.x;
    otc.y = (vr_pos.y + th * vr_vwr.y * 5.0) / b_ext.y;
    var tc: vec2<f32>;
    tc.x = in.vr_tex.x * 0.8 + tex_pos.x;
    tc.y = in.vr_tex.y * 0.5 + tex_pos.y;
    var a = dot(vr_vwr.xy, vr_vwr.xy);
    let to = vr_vwr.xy * a * a * 0.2;
    a = otc.y * otc.y;
    a = max(1.0 - a * 0.7, 0.0);
    var f = (b_ext.x - 30.0) * 0.001852;
    f = max(abs(otc.x) - f, 0.0) * (1.0 + f * 2.0);
    f = max(1.0 - f * f * 0.7, 0.0);
    f = a * f;
    tc += to * tex_ofs.y * 0.04;
    var i1 = emi(tc * 0.75);
    var cm = 1.0;
    var cf = 0.0;
    for (var i = 0; i < 8; i += 1) {
        let point = wall.colour_points[i];
        let cp = vr_pos.xy - point.xy;
        var d = dot(cp, cp);
        d = max(1.0 - d * 0.0005, 0.0);
        cf = cf + d * d * point.z * 0.5;
        cm = cm + d * point.z * 0.2;
    }
    i1 = i1 + min(cf, 1.8);
    i1 = i1 + max(2.0 - f * 4.0, 0.0);
    i1 = max(i1 - f * f * (fade + 0.1), 0.0);
    i1 = (i1 * (2.0 - i1) + i1) * 0.5;
    a = abs(i1 - 0.5) * 2.0;
    var b = abs(1.0 - abs(a - 0.5) * 4.0);
    b = 1.0 - abs(0.5 - b) * (0.5 + i1);
    b = b * (1.0 - a);
    i1 = (b * b + b) * 0.55;
    i1 = i1 * min(cm, 1.15);
    tc += vec2(0.333);
    tc -= to * tex_ofs.y * 0.1;
    var i0 = emi(tc);
    a = abs(i0 - 0.5) * 2.0;
    b = abs(1.0 - abs(a - 0.5) * 4.0);
    b = 1.0 - abs(0.5 - b) * 2.0;
    i0 = b * (1.0 - a);
    a = i0 * i1 * 2.0;
    b = 1.0 - 2.0 * (1.0 - i0) * (1.0 - i1);
    var c = clamp((0.5 - i1) * 10.0 + 0.5, 0.0, 1.0);
    a = mix(b, a, c);
    a = a * 0.5 + 0.5;
    a = (a * a + a) * 0.5;
    let alpha = (f * 1.5 - 0.4 - fade * 2.0) * 0.5 * (1.0 - a);
    f = 1.0 - f;
    f = f * f;
    a = max(a - f * f * 2.5, 0.0);
    a = a * (0.95 + f * f * 4.0);
    b = max(a - 0.5, 0.0);
    b = b * b;
    b = b * b * 4.0;
    a = a * 0.4 * (1.0 - b) + max(1.0 / (1.3 - a) - 1.0, 0.0) * b;
    a = a * (1.0 - fade * fade);
    let fcol = vec3(0.45, 0.33, 0.95);
    c = clamp((0.5 - a) * 10.0 + 0.5, 0.0, 1.0);
    var col = vec3(1.0) - (vec3(1.0) - fcol) * (1.0 - a) * 2.0;
    col = mix(col, fcol * a * 2.0, c);
    th = th * (1.0 - a) * 0.05;
    col = col * (1.0 + th);
    col = max(col - vec3(0.05), vec3(0.0));
    let out = vec4(col * col * 0.75, alpha);
    return clamp(out, vec4(0.0), vec4(1.0));
}
