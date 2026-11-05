## Context

The original's Ground Query walks its sector's static mesh list and body
list and tests each node's second flag word. Exarust's level is the rapier
world's triangle-mesh colliders, built from Bevy mesh entities; the node
words do not reach them.

## Decisions

- The flag rides on the mesh entity as a marker (`FloorMesh`), set where
  the node becomes entities (the model spawn and the tile grid's piece
  spawn), and the collider build records the colliders built from marked
  meshes by index and generation in `StaticColliders` (already a copied
  resource of the simulation bridge). No new fixed-step type is added.
- The tile grid's cell floors: Exarust spawns the floor set's pieces and the
  wall pieces' `+F` strips as parts (the same parts that feed the GI
  floor), where the original merges them into one `+Floor` node with the
  bit; those parts are marked.
- An instance node uses its own header word (no shipped instance node
  carries the bit).
- The grip mode: within a tick, the behaviour's clip start comes before
  the locomotion routine and the locomotion's own walk start after it, so
  `tick_mode` lets the locomotion's write win unless the start is the walk.
  The grip runtime's mode pass now runs after `CharacterTicks` and before
  `BodySystems`, where the original's writes sit relative to the step.
