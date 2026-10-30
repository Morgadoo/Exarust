// Rain streaks, as the original's rain program draws them: each drop is a
// line from its head to its tail. The vertex colour is the drop's colour
// bytes read as unnormalised numbers (160 for the grey, 0 at the head's
// alpha and 255 at the tail's), times `RainCol0`, the alpha faded by
// (0.6 − 0.000667·d)⁴ with d the drop's distance from the eye in cm. The
// fragment is the frame behind the streak at 75 % plus three times its
// squared excess over 0.1, plus the rain's colour, alpha-blended over the
// frame. The original's frame and colour buffer are 8-bit, so the frame
// read and the output are clamped to 1. Rules: docs/RENDERING.md (Rain).

import bevy_pbr::render::{
    forward_io::{Vertex, decompress_vertex},
    mesh_functions,
    mesh_view_bindings,
    view_transformations::position_world_to_clip,
};

// `RainCol0`: the rain colour and alpha (`exa_game::effects::rain`).
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> rain_colour: vec4<f32>;
// A debug aid (`EXA_RAIN_DEBUG`): x = 1 draws every streak opaque magenta,
// frame copy or not; x = 2 draws the frame copy opaque times 4, or green
// in a view without the copy.
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> rain_debug: vec4<f32>;

const CM_PER_METRE: f32 = 100.0;

struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(0) colour: vec4<f32>,
}

@vertex
fn vertex(vertex_in: Vertex) -> Varyings {
    var out: Varyings;
    let v = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
    let world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(v.position, 1.0));
    out.position = position_world_to_clip(world.xyz);
    // A copy of the line one target pixel to the right per copy index, so a
    // supersampled target's streak is one window pixel wide, as the
    // original's per-corner line is.
@if(VERTEX_UVS_A) {
    out.position.x += v.uv.x * 2.0 / mesh_view_bindings::view.viewport.z * out.position.w;
}
    var bytes = vec4(160.0, 160.0, 160.0, 255.0);
@if(VERTEX_COLORS) {
    bytes = v.color;
}
    var d = length(world.xyz - mesh_view_bindings::view.world_position) * CM_PER_METRE;
    d = d * -0.000667 + 0.6;
    d = d * d;
    d = d * d;
    out.colour = vec4(rain_colour.rgb * bytes.rgb, rain_colour.a * bytes.a * d);
    return out;
}

// A view without the frame copy (no screen-space transmission on its
// camera: the off-screen and interface cameras) draws no rain.
@fragment
fn fragment(in: Varyings) -> @location(0) vec4<f32> {
    if rain_debug.x > 0.5 && rain_debug.x < 1.5 {
        return vec4(1.0, 0.0, 1.0, 1.0);
    }
    var tex = vec3(0.0);
@if(VIEW_TRANSMISSION_TEXTURE) {
    let size = vec2<f32>(textureDimensions(mesh_view_bindings::view_transmission_texture));
    let tc = in.position.xy / size;
    tex = textureSampleLevel(
        mesh_view_bindings::view_transmission_texture,
        mesh_view_bindings::view_transmission_sampler,
        tc,
        0.0,
    ).rgb;
}
@if(!VIEW_TRANSMISSION_TEXTURE) {
    if rain_debug.x > 1.5 {
        return vec4(0.0, 1.0, 0.0, 1.0);
    }
    discard;
}
    if rain_debug.x > 1.5 {
        return vec4(tex * 4.0, 1.0);
    }
    tex = clamp(tex, vec3(0.0), vec3(1.0));
    var blm = max(tex - vec3(0.1), vec3(0.0));
    blm = blm * blm;
    tex = tex * 0.75 + blm * 3.0;
    return clamp(vec4(tex + in.colour.rgb, in.colour.a), vec4(0.0), vec4(1.0));
}
