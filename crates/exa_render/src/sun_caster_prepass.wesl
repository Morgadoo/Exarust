// The sun map's caster fragment for opaque materials without a prepass
// shader of their own (the terrain): the depth with the caster pass's
// polygon offset (`sun_caster.wesl`). Rules: docs/RENDERING.md (Sun shadow
// fit).

import bevy_pbr::prepass::io::VertexOutput;
import exa_render::sun_caster::caster_depth;

@group(constants::MATERIAL_BIND_GROUP) @binding(130) var<storage, read> exa_lights: array<vec4<u32>>;

struct CasterDepth {
    @builtin(frag_depth) depth: f32,
}

@fragment
fn fragment(in: VertexOutput) -> CasterDepth {
    var z = in.position.z;
@if(UNCLIPPED_DEPTH_ORTHO_EMULATION)
    z = in.unclipped_depth;
    var out: CasterDepth;
    out.depth = caster_depth(z, exa_lights[0]);
    return out;
}
