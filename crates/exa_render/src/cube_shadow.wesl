// Casters into Exarust's cached cube shadow maps, as the original's caster
// program writes them: the distance from the light, linear, divided by the
// light's radius R and the caster's depth scale k_c, so the map holds
// distance / (R·k_c) in [0, 1]. Alpha-cut materials (cobwebs, grates) cut
// their shadow the way they cut their colour. Rules: docs/RENDERING.md
// (Cached cube shadows).

struct Draw {
    world_from_local: mat4x4<f32>,
    clip_from_world: mat4x4<f32>,
    // xyz the light (metres), w 1 / (R·k_c).
    light: vec4<f32>,
    // x the material's opacity, y 1 when the texture is an opacity map (its
    // alpha squared) rather than the diffuse map (its alpha as is).
    alpha: vec4<f32>,
}

@group(0) @binding(0) var<storage, read> draws: array<Draw>;
@group(1) @binding(0) var cut_texture: texture_2d<f32>;
@group(1) @binding(1) var cut_sampler: sampler;

// The alpha cut of masked materials, as `exa.wesl`.
const CUTOFF: f32 = 0.5;

struct Vertex {
    @builtin(instance_index) draw: u32,
    @location(0) position: vec3<f32>,
@if(ALPHA_CUT)
    @location(1) uv: vec2<f32>,
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
    let world = (*d).world_from_local * vec4(v.position, 1.0);
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
    let texel = textureSample(cut_texture, cut_sampler, in.uv).a;
    let a = (*d).alpha.x * select(texel, texel * texel, (*d).alpha.y > 0.5);
    if a < CUTOFF {
        discard;
    }
}
    return clamp(distance(in.world, (*d).light.xyz) * (*d).light.w, 0.0, 1.0);
}
