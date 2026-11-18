## Why

Lane `renderphase` (roadmap B4, port queue Q17/Q18). The world-draw lanes
left the render-graph items that Bevy's batched phases could not take: the
doorway scissor on the opaque and alpha-tested lists, the cut-away faded
pass's depth-first order, and the grass hook's draw after every opaque and
alpha-tested mesh with depth writes off. Reading the frame walk again also
showed that no alpha-tested surface was cut in the colour pass (the main
shader had no discard; `AlphaMode::Mask` only picked Bevy's phase).

## Evidence

- The scene pass (asm/pseudo-code): alpha-tested list, opaque list, hook
  list after the opaque lists (the terrain's lit draw), depth writes on,
  the grass hook's list and one more, depth writes off, decals and blended
  lists under the list bias, a hook list, then the faded pass over the
  renderer's own opaque and alpha-tested lists (mask `0x60000000` single
  view, `0x20000000` super-sampled: depth half with colour masked, then the
  colour half at one less each entry's fade, blend `0x203`), the weather
  and the closing hooks. Earlier notes read the faded pass as the
  particles' pass.
- The grass hook registers in the renderer's second depth-write hook list
  (priority 20) and draws with the depth test on and depth writes off;
  `shadegrs_f` drops a texel below 0.1 alpha. The alpha-tested programs
  (`Shd.AlphaTst`) drop a diffuse texel below ½ (`shadeopt_f`,
  `shadetrl_f`; `MatDif.a·a` in `shadesss_f`; the hair alpha in
  `shadehar_f`).
- Worker GPU pairs: `context/artifacts/2026-11-19-renderphase/` (pairs1: e01, e02 and the hub within 0.02 % of pixels, arenao3 2.4 % from the grass cut).

## What changes

- `passes::PhaseList`: the transparent phase in list blocks (scissored
  alpha-tested, scissored opaque, grass hook, decal, blended, late).
- `sector_walk::move_scissored_opaque` moves the visible scissored opaque and
  alpha-tested meshes of the scene view to the transparent phase with their
  own pipelines; the scissor items clip them; a mesh leaving the scissor is
  re-queued through Bevy's dirty specializations.
- `ExaMaterial::alpha_tested` and `late` (`LateDraw`): the alpha test
  compiled in (`EXA_ALPHA_TEST`), the grass hook (unblended, depth writes
  off) and the faded pass's depth and colour halves.
- `cut_away_live`: a fading part's copy draws as the colour half; a depth
  child lays its depth first.
- The terrain plants draw in the grass hook block.
- `passes`: the particle pass model renamed to the faded pass it is.
- The rain draws its reflected set (a second entity, the reflected turn)
  above shading quality three, as each sub-frame pass of the original does
  (the frame routine tests the shading quality's global).
- `ExaPost` carries the view's blur kernel (`Filter` and its last index,
  `bloom_kernel` for its tap count: 16 for the game view, 24 for the item
  renderer's) and the blur passes loop over it.
