// Casters into Exarust's cached cube shadow maps, as the original's caster
// programs write them: the distance from the light, linear, divided by the
// light's radius R and the caster's depth scale k_c, so the map holds
// distance / (R·k_c) in [0, 1]. Surfaces cast the alpha-tested way
// (cobwebs, grates) drop the texels whose cut texture has an alpha below ½,
// as the original's alpha-tested caster does, whatever their opacity.
// Skinned casters (characters) are posed by their joints: each vertex is
// the weighted sum of its joints' matrices (joint × inverse bind pose).
// Rules: docs/RENDERING.md (Cached cube shadows).

struct Draw {
    world_from_local: mat4x4<f32>,
    clip_from_world: mat4x4<f32>,
    // xyz the light (metres), w 1 / (R·k_c).
    light: vec4<f32>,
    // x: a skinned caster's first joint matrix in `joints`.
    joints: vec4<u32>,
}

@group(0) @binding(0) var<storage, read> draws: array<Draw>;
@group(0) @binding(1) var<storage, read> joints: array<mat4x4<f32>>;
@group(1) @binding(0) var cut_texture: texture_2d<f32>;
@group(1) @binding(1) var cut_sampler: sampler;

// The alpha-tested caster's cut.
const CUTOFF: f32 = 0.5;

struct Vertex {
    @builtin(instance_index) draw: u32,
    @location(0) position: vec3<f32>,
@if(ALPHA_CUT)
    @location(1) uv: vec2<f32>,
@if(SKINNED)
    @location(2) joint_index: vec4<u32>,
@if(SKINNED)
    @location(3) joint_weight: vec4<f32>,
}

struct Varyings {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) @interpolate(flat) draw: u32,
@if(ALPHA_CUT)
    @location(2) uv: vec2<f32>,
}

@vertex
fn vertex(v: Vertex) -> Varyings {
    let d = &draws[v.draw];
    var world_from_local = (*d).world_from_local;
@if(SKINNED) {
    let o = (*d).joints.x;
    world_from_local = v.joint_weight.x * joints[o + v.joint_index.x]
        + v.joint_weight.y * joints[o + v.joint_index.y]
        + v.joint_weight.z * joints[o + v.joint_index.z]
        + v.joint_weight.w * joints[o + v.joint_index.w];
}
    let world = world_from_local * vec4(v.position, 1.0);
    var out: Varyings;
    out.clip = (*d).clip_from_world * world;
    out.world = world.xyz;
    out.draw = v.draw;
@if(ALPHA_CUT)
    out.uv = v.uv;
    return out;
}

@fragment
fn fragment(in: Varyings) -> @builtin(frag_depth) f32 {
    let d = &draws[in.draw];
@if(ALPHA_CUT) {
    if textureSample(cut_texture, cut_sampler, in.uv).a < CUTOFF {
        discard;
    }
}
    return clamp(distance(in.world, (*d).light.xyz) * (*d).light.w, 0.0, 1.0);
}
