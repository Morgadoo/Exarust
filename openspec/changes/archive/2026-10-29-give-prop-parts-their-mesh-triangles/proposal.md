# Proposal

## Why

The pair stage already ran the original's sphere-against-mesh query, but no
loader handed a part its triangles, so every body pair ran the sphere
fallback. The original's contact generation calls the touched body's scene
node's own mesh queries (its sphere and swept-sphere queries over the
node mesh), so a prop is met through its mesh. A DeepSeek lane
(simsector7) passed the node mesh to prop parts; its review kept the mesh
copy to nodes whose body registers (the lane copied every mesh node's
triangles at load, level geometry included) and corrected the docs.

## What Changes

- `exa_assets::model::mesh_triangles` and `ModelNode::triangles`: a node
  with a registering body keeps its mesh's triangles in its frame (metres),
  `None` for an index past the positions or an empty list.
- `exa_physics::body::RigidPart::triangles`, handed to the solver part by
  `Ragdoll::rigid`; `exa_world::bodies::register_bodies` passes the node's
  triangles, so a prop is the mesh side when it is the less mobile part.

## Capabilities

### Modified Capabilities

- `physical-body`: prop collision mesh (added).

## Impact

- `crates/exa_assets/src/model.rs`, `crates/exa_physics/src/body_rigid.rs`,
  `crates/exa_physics/src/body_pairs.rs` (doc), `crates/exa_world/src/bodies.rs`,
  struct literals in tests across the workspace, crate READMEs.
- Evidence: the sector's contact generation calls the touched body's node
  sphere query (static path) or swept query (swept path) after the mesh box
  and mesh sphere pre-tests; `context/re/body-pairs.md` §6;
  `context/re/packs/simsector`.
- Out of scope: character part triangles (the character builder has only
  asset handles), the mesh kd-tree walk order (triangles are visited in file
  order), the per-part kd-tree as a cost cut, weapon meshes.

## Phase

Phase 3, bodies: physics (contacts between bodies).
