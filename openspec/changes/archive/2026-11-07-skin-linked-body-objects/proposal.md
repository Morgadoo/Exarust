# Proposal

## Why

The original's importer hands a root mesh with render bit `0x400000` to the
skin builder: the object's mesh nodes become the bones of one skin
controller, their meshes are merged into a `_SKIN` mesh drawn in their
place, and the skin follows the nodes that the linked rigid bodies move. 22
shipped objects use it (sacks, mattresses, flails, hanging growths). Exarust
drew each part rigidly, so a moved sack or mattress opened gaps between its
pieces and hanging growths showed their segments.

Evidence (REA, `Exanima.exe`): the finalise pass's skin branch, the `_SKIN`
builder, its bone callback (bones numbered in the walk, inverse bind from
the node's frame), the walk over marked children and the merge append; a
corpus test over the shipped object libraries.

## What Changes

- `exa_assets::prop_skin`: skin roots, the bone walk, rest frames and
  inverse binds; `ModelNode::prop_skin` with the members' skinned meshes.
- `exa_world::prop_skin`: spawned members drawn as Bevy skinned meshes
  over the model's node entities.
- `geometry_node::SHADOW_ALL` renamed `VERTEX_DELTAS` (cloth's delta
  stream).

## Impact

`exa_assets`, `exa_world` and the `ModelNode` literals in other crates;
docs/formats/RFC.md, the crate READMEs. Physics is unchanged.
