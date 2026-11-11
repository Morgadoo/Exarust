## Why

TRenderer gap lane (round 195). The scene pass (the largest TRenderer
routine) draws the decal list and the blended lists under a projection
depth bias that Exarust only modelled as a constant: no surface was drawn
with it. Recent lanes found backlog premises wrong, so the TRenderer, GL
immediate-draw helper and FLX_GPUPrograms routines were re-listed first;
the bias was the one reachable scene-pass behaviour still missing.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The scene pass, after depth writes go off and the depth test is on,
  loads the first view's projection `[2][2]` term once, then for every view
  (the sub-view count) widens it to double, adds the double constant
  2·10⁻⁵, narrows it to single and stores it in both the view's projection
  and its cropped sub-frame projection, rebuilding each composite; it draws
  the decal list, a hook list and the blended list (per sector when the
  scene keeps sectors), then stores the unbiased first-view term back into
  every view and rebuilds the composites before the next hook list and the
  particles.
- The camera's frustum term is `−(f + n)/(f − n)` with n = 100 and
  f = 12 500 engine units; in single precision the stored step is
  2.0027·10⁻⁵.
- Premise checks: the material compile slot logs "Attempt to compile a
  removed material" and skips the material; it builds no fallback shader
  (the backlog's compile fallback does not exist). The renderer's full
  material rebuild overrides the alpha-tested, opaque and blended entries'
  colour words with fixed greys only when a renderer byte is set that no
  0.9.5 path was found to write.

## What Changes

- `exa_render::passes`: `LIST_PROJECTION_BIAS` (double), `biased_depth_term`
  and `list_depth_shift` (the bias as a change of inverse view distance for
  the original's planes, about 9.93·10⁻⁶ per metre).
- `exa_vertex.wesl`: the surface material's vertex stage, Bevy's mesh
  vertex stage plus the bias under `EXA_LIST_DEPTH_BIAS`, added to Bevy's
  reverse depth scaled by the view's near plane.
- `ExaMaterial`: a pipeline key bit for surfaces with a decal or blended
  render state; `--no-list-depth-bias` turns it off, to compare.

## Impact

- `crates/exa_render` (passes, material, exa_vertex.wesl, shader_reload),
  `crates/exarust` (option), `docs/RENDERING.md`, the crate README.
- No simulation change: lockstep traces identical; shots and FPS unchanged
  against round 194.
