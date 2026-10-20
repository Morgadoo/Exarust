# Proposal

## Why

The original fits its sun shadow map around the smallest sphere that holds the clipped view region. Exarust has no such solver, so the fitted map cannot be rebuilt as the original computes it, down to the bits of its double-precision run.

## What Changes

- A pure-maths module computes the smallest enclosing sphere of a point set as the original does: widened to f64, move-to-front over a support set of one to four points, its candidate orders, slack and degenerate cases, narrowed back to f32.
- The vector helpers it uses (f32 to f64 points, f64 to f32 point, f64 lerp) come with it.
- No points leave the centre unwritten and set the radius to 4000.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: the smallest enclosing sphere of the sun shadow fit.

## Impact

- `exa_render::bound_sphere` (new); `docs/RENDERING.md` (Sun shadow fit) and the crate README.
- Evidence: REA/Ghidra on Exanima.exe: the sphere driver, its four support handlers, the 2-, 3- and 4-point spheres, the containment test and the three vector helpers, read in pseudo-code and checked against the SSE2 assembly for the order of every sum and the NaN side of every comparison; the candidate tables and constants read from the image. No corpus data.
- Out of scope: the shadow fit itself (clipping the view by planes, the light-space transform, the 200 cm radius steps with hysteresis and texel snapping) and replacing Bevy's cascaded sun shadows with the fitted orthographic map. Until that lands nothing calls the solver at run time.

## Phase

Phase 5, lighting fidelity: the sun's shadow map.
