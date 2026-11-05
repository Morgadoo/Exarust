## Why

Backlog system S10. Exarust's bodies kept CPU triangles only for props. Character limbs and weapons were met through their collision spheres. No mesh had a kd-tree on the game path. The sector's body and static-collider lists were not kept. Several callers in the original read these:
- the pair stage's mesh narrow phase;
- the Height Query's mesh mode in the ground placement's leg snap;
- the picks on characters;
- the NPC search's gathering.

Phase 3 (physical body; TSimSector, TkdTree, RF_kdTrees). No docs/PLAN.md edit in this change (the main session owns it).

## What the original does

Evidence: REA (Ghidra on `Exanima.exe`), checked against the assembly.

- **Which meshes get a tree.** The scene loader builds a mesh node's triangle records when the node carries the collision flag `0x200000` and not the opt-out `0x4000`. It then queues the mesh for the kd-tree builder thread when the mesh has more than 10 and fewer than 100 000 triangles. The thread installs the finished tree's query callbacks into the mesh and all its instances, between steps only. Until then, and for any mesh outside the bounds, the mesh's own callbacks test every triangle in order.
- **Shape builders.** The limb, root and weapon shape builders build the records and a tree for the mesh they fit when it has none, at any triangle count. So every character limb carries its node mesh and a tree. The tree is built once, in the node's frame. A skinned limb is queried in its part's frame, so nothing is rebuilt as the body moves.
- **Sphere query.** It descends from the root. A sphere wholly past a split plane (`plane < c − r`) goes to the second child only. One reaching it (`plane ≤ c + r`) defers the second child on the tree's own stack and goes to the first. Any other goes to the first (a NaN centre takes the first). Each non-empty leaf runs the per-triangle sphere test, which re-reads the sphere after every hit.
- **Swept query.** It sets the direction's reciprocal up (1e6 for a component at or under the single-precision epsilon). It walks the leaves along the swept sphere, with the record's length as the walk's limit. The limit is put back to the best before each leaf. A hit's callback refreshes the record (end, length, direction, reciprocal), and the descent reads it live.
- **Height Query modes.** The scene word `0x2000000` is set by the grid environment, so every game level has it. With it, the Height Query walks each sector's nine cells around the query: the static grid's meshes, then the body grid's bodies. It skips a body out of the world (`0x800`) and a living character's limb (family `0x12000000`, character not dead). Each mesh is pre-tested (segment against its box, swept sphere against its bounding sphere), then asked through its swept nearest query in its own frame. Without the word, it casts a ray over the flat lists.
- **Sector grids.** A sector keeps its bodies in a 6 m grid by the cell of each mesh box's world centre, and its static meshes in the static grid.
- **The NPC search's gathering** walks, in each flooded cell, the body list and then the static-collider list.

## What Changes

- New `exa_physics::mesh_tree`: a node mesh's tree, built in the original's centimetres over the part's frame, with the original's count rule (`takes_tree`) and the shape builders' any-count rule (`shape_tree`). Trees are shared per triangle list. It provides the sphere, live swept, ray and column queries, and `body_drop` for one body mesh.
- `KdTree::sphere_leaves` and `KdTree::sweep_live` (with `SweptRecord` and its refresh) port the two tree walks.
- `Part::tree`: the pair stage's mesh query walks the tree when the mesh side has one. A mesh of ten triangles or fewer still tests every triangle in order.
- Character limbs get their node mesh at body build. Their trees are built on the async compute pool and installed when done; until then the triangles answer.
- Held items get their node meshes, and factory weapons their merged mesh.
- The hand's character pick asks each limb's tree for the nearest front face, with the ray in the limb's frame. A limb without a tree falls back to its spheres. Logged at debug ("mesh pick: … (bone) face …").
- New `exa_physics::sector_lists`:
  - `SectorBodies`, the body grid, rebuilt after each body step;
  - `StaticColliders` records each collider's mesh entity;
  - the Height Query's mesh mode (`MeshLevel`, `height_meshes`), which the player's ground placement now uses.
- The NPC search's objects come from the sector lists. In each cell come the body grid's parts, then the static grid's meshes, each mapped to its placed object. A placed object with neither is never gathered.

## Out of scope

- The original's builder thread and its rings (`kdtree::Ring`). Exarust builds on the async pool and installs from a frame system.
- The dressed character's meshes. A body built after a refit takes whatever meshes `part_box` bounds.
- Per-particle cloth culling against body meshes.
- The Height Query's ray mode for scenes without the grid word.
- The static-collider mesh pick through trees. The level is still answered by rapier's trimesh.
- The skipped family `0x10…` with bit `0x1000` in the Height Query.
