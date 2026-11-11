// The original's smoke and mist program (`shadesmk_f`), which every
// material record of type 2 draws with: its start-up hook puts the smoke
// draw in the type table's third slot. The surface is black; its alpha is
// the facing ratio, the opacity texture read twice at the scrolled
// `UVOfst` (a constant quarter without one) and the vertex colour's alpha.
// `exa.wesl` samples the textures and calls these; plain WGSL (no imports)
// so the Rust tests can validate it. The formulas are mirrored and pinned
// in `exa_render::smoke`. Rules: docs/RENDERING.md (Smoke and mist).

// The scroll's cycle, seconds (80 000 ms).
const SMOKE_SCROLL_PERIOD: f32 = 80.0;
// The owner node's scale the scroll starts from: the node default, 1.
const SMOKE_NODE_SCALE: f32 = 1.0;

// `UVOfst` at scene time `time` (seconds): (0.5, t + 0.5, t) with
// t = the node's scale + the time's fraction of the 80 s cycle.
fn smoke_offset(time: f32) -> vec3<f32> {
    let t = SMOKE_NODE_SCALE + fract(time / SMOKE_SCROLL_PERIOD);
    return vec3(0.5, t + 0.5, t);
}

// The program's alpha: a = |N·V|·1.5 − 1, plus the opacity term (the two
// scrolled reads' product, or 0.25 without an opacity texture), then
// (a − 1 + vertex alpha)·0.8.
fn smoke_alpha(ndv: f32, opacity: f32, colour_alpha: f32) -> f32 {
    let a = ndv * 1.5 - 1.0 + opacity;
    return (a - 1.0 + colour_alpha) * 0.8;
}
