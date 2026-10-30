# Model the super-sampling sub-frames

## Why

The per-draw routine's largest missing piece was the super-sampling view's
per-sub-frame fill: the jittered projections, the view matrices, their
composites, the object matrix pair a draw uploads, the sub-frame animation
clocks and the depth-lane rewrite of a geometry with the shutter flag. The
setting table was also short (levels 6 and 7 read as "off", the 3-sample
setting drew four sub-frames), the motion-blur rule was misread as a time,
and the per-draw emissive choice tested the wrong word.

This change advances **docs/PLAN.md Phase 5 (Visual and audio fidelity)**:
FLX_GPUPrograms and TShaderBuilder.

## What changes

- `exa_render::subframe`: projection from the stored half extents and a
  sample's jitter (through `engine_math::frustum`), the transposing view
  matrix, the engine-order product, the object pair, the viewport shifts,
  the animation clocks, the depth lane and the per-quarter shadow mode (from
  `view::SupersampleLevel::pass_state`).
- `exa_render::view`: the full eight-level table and the 124-entry jitter
  sequence as exact bit patterns; a sub-view's jitter read past its level's
  run as the engine reads it.
- `shader_program::supersample_factor`/`supersample_passes` from that table.
- `draw_state`: three render-object words, the liquid level's value, the
  shutter length per sub-frame, the stale flag surviving a program change,
  and the emissive choice by the render object's own word and scale.

## Evidence

The view's quarter loop (sample count and jitter base from an 8-entry
table of pairs; `min(samples, 4)` passes; jitter `2·half·j/size`; frustum
edges `half + d` and `−half + d`; descending pass loop building the view
and the `view · projection` product), the frustum routine (constants 2,
0, −1, −2 and the `comiss`/`jp` far-plane branch), the 4×3 transpose
routine (`0 − t` dotted with each axis row, summed `(x + y) + z`), the
4×4 product (`(a·b + a·b) + (a·b + a·b)` per row), the 4×3 point
transform, the per-draw routine (depth-lane rewrite from the first pass's
lane, the liquid point through the world placement, the stale byte, and the
emissive branch reading the render object's own word bit `0x20000` and its
own scale in both draw variants), and the 124 jitter pairs and the shadow
taps read from the executable's data.

## Out of scope

The GL calls (viewport, uniform uploads, binds and draws), the planar
reflection pass whose matrix pair takes the same lane rule, the voxel-GI
manager's per-object `VXGIPos`, and wiring: Exarust super-samples by
rendering at a larger scale, so the sub-frame fill and the draw plan have no
caller in the running renderer and frame and load time are unchanged. The
third copy of the level table in `exa_ui::viewport` is left to that crate.
