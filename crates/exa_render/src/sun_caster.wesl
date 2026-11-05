// The sun caster pass's polygon offset (`exa_render::sun_casters`): every
// caster fragment drawn into the sun map moves away from the sun by
// `factor · slope + units` of the window depth, `slope` the larger of its
// depth's two per-texel derivatives (GL's allowed approximation of the
// gradient's length). The factor and units sit in lanes z and w of the
// light records' first element (`exa_render::lights`); zero there, with no
// fitted map, leaves the depth alone. Bevy's map is reverse-Z, so away from
// the sun is down. Rules: docs/RENDERING.md (Sun shadow fit).

// The caster depth of a fragment at window depth `z`, with `header` the
// light records' first element. Call it in uniform control flow, before any
// discard: it takes the depth's derivatives.
fn caster_depth(z: f32, header: vec4<u32>) -> f32 {
    let slope = max(abs(dpdx(z)), abs(dpdy(z)));
    let factor = bitcast<f32>(header.z);
    let units = bitcast<f32>(header.w);
    return clamp(z - (factor * slope + units), 0.0, 1.0);
}
