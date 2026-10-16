// Blood stains on weapons, as the original's weapon-stain program draws
// them: the blood gradient, keyed on the mesh's position in the weapon's
// frame (cm), faded to white by the object's fade and multiplied into the
// frame. The depth is nudged toward the camera so the overlay passes the
// test against its own weapon. Rules: docs/RENDERING.md (Blood on weapons).

import bevy_pbr::render::{
    forward_io::{Vertex, decompress_vertex},
    mesh_functions,
    view_transformations::position_world_to_clip,
};

struct StainParams {
    // u = x·coords.x + coords.y, v = y·coords.z + coords.w (x, y in cm).
    coords: vec4<f32>,
    weapon_from_mesh: mat4x4<f32>,
    fade: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> stain: StainParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var gradient: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var gradient_sampler: sampler;

const CM_PER_METRE: f32 = 100.0;
// `exa_render::stain::STAIN_DEPTH_BIAS`.
const DEPTH_BIAS: f32 = 0.000005;
// The original's texture LOD bias on colour textures (exa.wesl).
const TEXTURE_BIAS: f32 = -0.75;

struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(0) gradient_uv: vec2<f32>,
}

@vertex
fn vertex(vertex_in: Vertex) -> Varyings {
    var out: Varyings;
    let v = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
    let world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(v.position, 1.0));
    out.position = position_world_to_clip(world.xyz);
    // Reverse depth: nearer is larger.
    out.position.z += DEPTH_BIAS * out.position.w;
    let p = (stain.weapon_from_mesh * vec4(v.position, 1.0)).xyz * CM_PER_METRE;
    out.gradient_uv = vec2(
        stain.coords.x * p.x + stain.coords.y,
        stain.coords.z * p.y + stain.coords.w,
    );
    return out;
}

@fragment
fn fragment(in: Varyings) -> @location(0) vec4<f32> {
    let texel = textureSampleBias(gradient, gradient_sampler, in.gradient_uv, TEXTURE_BIAS).rgb;
    return vec4(mix(texel, vec3(1.0), stain.fade), 1.0);
}
