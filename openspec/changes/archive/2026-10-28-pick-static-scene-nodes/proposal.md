# Proposal

## Why

docs/PLAN.md, Phase 7 (game modes and content), the arena hub: its click
hook was wired, but the hub's two notice boards (`\xaaArenaMach`) and the
shop counter (`\xaaArenaShop`) are static scenery with no rigid body. The
static colliders were built in batches that kept no scene node, so a click
on them picked the level and opened nothing. The original's cursor pick
walks the scene's nodes and returns the node whose mesh it met; the hook
compares that node's name.

## What Changes

- `exa_physics::StaticNode` marks the nodes `exa_world::spawn_model_nodes`
  spawns from a model; `ColliderNodes` keeps one entity per static collider
  built under such a node (no name copy). Generated level geometry keeps no
  entry.
- `hand::Hit::SceneNode { node }`: the hand's pick names a scene node's
  mesh; plain geometry stays `Hit::Level`. Walk-to, the cursor name, the
  hold, the reach and the drags treat a scene node as the level.
- The hub's click hook reads the scene node's stored name and side.
- A model's `_null` groups (never drawn) become bare position-and-index
  meshes kept in the main world, spawned hidden with `PickOnly`: their
  colliders (`PICK_ONLY`, no collision or solver groups) answer only the
  hand's `fixed_pick`. The shop counter's node holds only such a box.
- Review fixes: the lane had made `_null` geometry fully solid in every
  level (character controller, bodies' level contacts, sweeps and gameplay
  rays) and built it with the drawn meshes' full vertex streams, uploaded to
  the GPU; it is now pick-only, lean and never uploaded, and skinned
  meshes' `_null` seams are not built. In `play arenahub` the flow is
  switched to Exarust's practice stand-in once the player spawns, so the
  hook saw mode 2 and answered nothing; with the hub loaded that mode now
  counts as the arena. Checked in the running game: clicks on the two
  notice boards opened Manage Company and the match board, a click on the
  counter opened the trade window.

## Evidence

The library entry `shop_arena` in `Objlib.rpk` holds the node
`\xaaArenaShop` (class 0x89) whose only mesh group is `_null`: a click on
the shop in the retail game can only meet those faces, so the original's
pick meets `_null` geometry. The notice board nodes instance `noticead`.
The original's scene pick (class 0x89 mask, the no-pick bit, layers, bounds,
each mesh's nearest face) is described in docs/PLAYER.md.

## Out of scope

- Whether the original's solver collides with `_null` faces (not traced;
  Exarust keeps every level's collision as it was).
- Which node field the hub's side value is (world x stands in).
- The shop's first stock build, the match's return to the hub, the windows'
  close controls, the board's pictures and items.

## Phase

Phase 7 (game modes and content), the arena hub's wiring.
