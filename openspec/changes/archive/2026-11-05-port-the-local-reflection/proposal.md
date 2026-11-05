# Proposal

## Why

Backlog system S8 (the engine-faithful render pipeline) still listed a
planar reflection, the jittered super-sampled sub-frames and the
per-object `VXGIPos` as missing. Round 59 ported the sun caster pass. This
change traces the three remaining parts in the original and ports what
shows on screen.

Phase: the rendering milestone of docs/PLAN.md (render pipeline, S8); no
PLAN.md edit in this change.

Evidence (REA, Ghidra on `Exanima.exe`, checked against the assembly):
- **Reflection.** The executable has no mirrored camera and no clip
  plane: no `GL_CLIP_DISTANCE` enable, no `gl_ClipDistance` or clip uniform
  in any of the 54 decoded shader programs, and the view's second matrix
  set is the scissor region's sub-projection, not a mirror. What the
  programs call a local reflection (`Shd.LocalRef`) is the `reflection`
  template: a read of the scene's low field (the post-processor's
  quarter-size blurred bright pass, which the material writer binds for a
  `0x400000` shader word) pushed along the view-space normal. The post
  chain runs at the end of the frame, so the read is of the previous
  frame. `ViewPrj`, which scales the push, is the view's projection
  element `[0][0]` × 500 and that times the viewport aspect: the point
  projection divides by the same field, and the frustum's corners read it
  beside `[1][1]`, `[2][2]` and `[3][2]`.
- **Which surfaces.** The factory sets `LocalRef` for a record with a
  reflection layer and extra flag `0x800` above shading quality 4. In the
  shipped libraries only `glsdark02` (common.rml) qualifies: the dark
  glass of the lamp globes (arenas, exanima01 to 04), the hourglass, the
  glasses and the sewer lamps. The sewer's water (round 60) is the
  terrain's wet floor gloss, which reflects the environment cube; no
  surface draws a water plane.
- **Sub-frames.** The 3D view's frame routine draws all four sub-frames
  of a pass in one renderer walk: each sub-frame has its own viewport in
  a quadrant of the double-size buffer, its own jittered projection and,
  for an animated camera, its own clock; the resolve averages them. The
  rain's per-corner and reflected passes and the split shadow taps
  follow the sub-frame.
- **`VXGIPos`.** The lit-material writer passes the node's world
  bounding-sphere centre (the sphere the visibility test reads) to the GI
  manager's lookup, not the node's origin; node flag `0x800000` (and the
  terrain and grass callers) drop the half voxel.

## What Changes

- `exa_render::local_reflection`: the low field (`SceneLowField`, a
  quarter of the post camera's target, sRGB), copied from the post
  chain's blurred texture after each frame's post pass, bound to every
  local-reflection material (`ExaMaterial::scene_low`, binding 133/134,
  `flags2::LOCAL_REFLECTION`); `exa.wesl` mixes the GI-volume cube
  reflection toward the read by `min(2·gloss, 1)²·(1 − N·V)`.
  `EXA_LOCAL_REF_DEBUG=1` shows the read alone.
- `exa_assets::model`: the factory's `LocalRef` bit sets the flag.
- `shader_uniforms`: `ViewPrj` is the projection's scale × 500 cm (it was
  read as the frustum's half extents).
- `exa_render::gi_tag`: each surface's GI block cell from its world
  bounds centre, carried in the mesh tag's bits 20–31 (`GiCellTag`); the
  light-list slot keeps bits 3–19. The surface shader picks the block by
  that cell and falls back to the origin without one.
- `supersample`: `EXA_SUBFRAME=0..3` shows one sub-frame of the resolve.
- Docs: RENDERING.md (Local reflection, Supersampling, Voxel GI),
  DEBUGGING.md, the exa_render README.

Not changed, by decision: the scene is still drawn once at twice the
window's size (an ordered 2 × 2 grid) rather than as four jittered
sub-frames; see design.md.
