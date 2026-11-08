# Design

Extends [docs/RENDERING.md](../../../../docs/RENDERING.md) (Pass order,
Sectors on the live path) and the `scene_nodes` model.

## Behaviour

- One scene per loaded tile-grid level: the scene's own sector, then the
  rooms appended through the batch insertion in room order. A room
  sector's portals are its door links' quads (shared edge, −50 cm up to
  the opening's top), facing the room behind, leading to its sector; the
  linked sectors are those rooms.
- Every main-world mesh entity (not a room shell, not on an off-screen
  layer) is a mesh node: in its room's sector, in the sector of the room
  at its moving root's position (characters), or in the scene's own.
  A room change moves it (`move_to_sector`); a despawn queues its removal;
  a transform or bounds change refreshes its frame. A shadow-casting mesh
  carries the bounds split bit.
- Bounds: a sector rebuilds its boxes when a mesh joins or leaves, or when
  a moved shadow caster leaves its first box; the scene bounds follow.
- Each frame: room states (the scene's own sector active), the main
  camera's view volume, `Scene::frame`. A registered mesh with no draw bit
  leaves the main camera's `VisibleEntities` after Bevy's visibility
  check; `NoFrustumCulling` meshes and shadow views are untouched.
- The renderer: after Bevy's transparent sort, a stable re-sort by (list,
  sector) for the decal and blended lists; the list is read back from the
  sort value's block (`order_bias`).

## Deviations

- Exarust's mesh entity is one part; the original's mesh node holds a
  model's parts. Each entity is its own node.
- The portal scissor is not a pixel scissor; only the portal frusta cull.
- The per-quadrant scissor regions of the super-sampled sub-frames are not
  reproduced.
- Lights are not registered (Exarust's lights stay global).
- Within one state and sector Bevy's distance order stays.
- `quiet_passes`: a hidden sector whose lists did not change visits only
  the meshes refreshed since its last pass (same masks and cells; the
  frame's moved list is in node order).
- When bounds are rebuilt is an Exarust choice (the original's trigger was
  not traced).

## Evidence

Grid runs (context/artifacts/2026-10-30-sectors): before/after shots,
frame-profiler FPS, the scene's own counts and timing, lockstep traces.
