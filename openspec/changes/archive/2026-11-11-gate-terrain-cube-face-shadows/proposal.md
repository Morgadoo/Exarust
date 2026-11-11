## Why

PLAN Phase 2 (outdoor levels). FLX_Terrain_Texrender2 (unit routines) and
TStaticTerrain gap lane (round 186, tag texrender). Gaps named by the last ratings:
the texture unit's Gaussian blur kernel is built but never used live, the cube-face
terrain shadow draw is not gated by the main-view bit (left out in round 166 because
of Exarust's cached cube faces), and the composite lit draw is opt-in.

## Evidence

REA (Exanima.exe), checked in the assembly and by a scan of every RIP-relative
displacement in `.text`:
- The texture unit's init builds the radius-120 Gaussian, widens it to four floats an
  entry into a global kernel, frees the narrow copy, and allocates three planes: a
  481 x 481 float plane, a plane of 482 rows of 244 floats, and a 244 x 241 float plane.
  No other code references the kernel, its count or the first two planes. The third
  plane is read by the CPU material-rule pass as rule input 2 and is never written.
  The blur is therefore not applied anywhere in 0.9.5, and input 2 stays zero, as
  Exarust already models it.
- The one-block terrain shadow draw (a light's cube face) skips a block node with the
  external-draw bit and draws only one whose visible word has bit 0 (the main-view
  bit). It writes no dirty mask. A cube face renders only when it is needed and dirty
  (the light changed, or a linked caster moved or changed), so the original keeps a
  face as rendered whatever a block's visible bit does afterwards.

## What Changes

- exa_render: `cube_shadows::CubeCasterGate`, the casters a face rendered this frame
  leaves out (it does not dirty the face); `CubeStats::gated`.
- exa_world: `terrain::shadow_draw::cube_faces_skip_undrawn_blocks` lists the terrain
  blocks the live scene's visible pass left undrawn, after the scene's run and before
  the cube faces are built (`EXA_TERRAIN_SHADOW_ALL=1` turns it off with the sun rule).
- Docs: RENDERING.md (Terrain shadows), formats/TERRAIN.md (the unused blur), the
  exa_world and exa_render READMEs; `blend::gaussian_kernel` documents why it is unwired.

## Out of scope

The composite lit draw stays opt-in (`EXA_TERRAIN_SETS=1`): on arenao1, arenao3 and
arenao4 with a fixed seed its picture matches the per-pixel draw away from the camera
and its FPS is the same within noise, but the ground near the camera is visibly softer
(the sets are sampled at level 0 of a 1024 texel set per 6 m block). No retail outdoor
shot exists to tell which matches the original; past lanes could not reach an outdoor
arena in retail.
