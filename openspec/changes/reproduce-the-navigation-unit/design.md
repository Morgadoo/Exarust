# Design

Extends `docs/PLAYER.md` (Camera; cursor pick) and the terrain vegetation
rules of `docs/formats/TERRAIN.md` (no format change).

## Decisions

- The unit stays plain functions (`nav_camera`, `nav_pick`); the chase
  camera keeps the controller's state. Only the align wrap, the held
  pitch and the orbit rate are on the live path; the placement, preset,
  free zoom, kick and cut-away segment are models with tests.
- Arithmetic follows the listing, not the pseudo-code: sums in the
  instructions' order, single versus double precision where the
  instructions convert, MAXSS/MINSS operand order, unordered compares.
- The engine's planar yaw and arc cosine are reproduced in `nav_camera`
  (the head-tracking atan2 in `exa_combat` is a different polynomial for
  positive cosines). The engine's sine and cosine are copied into the
  vegetation module (it cannot depend on `exa_game`).
- Mesh callbacks are a trait. The physics world's fixed colliders answer
  for the level; the cursor wraps it in a node whose bounds hold the whole
  segment, so the reach is the physics cast's, as before.
- The cut-away bits live in the scene word, where the original keeps them
  (the same word as the owned and in-scene bits); the probe stops at the
  bounding sphere where the original would ask the mesh.

## Open questions

- Which reader uses the preset's retention and rate roots, and the
  extent step: the camera's tick routine, outside this unit; answered by
  REA on the controller's tick when the preset is wired.
- The kinesis pick's segment callback and its distance write-back: the
  mesh unit's kd-tree query (REA on the mesh class), with no CPU triangles
  in Exarust yet.
- The kick's callers' directions and sizes: the motile impact routines
  (REA), before the kick is fired.
- A retail capture of the grass points to confirm the scatter end to end.
