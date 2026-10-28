# Design

## Node map

A static collider is built from one mesh copy; at copy time the build walks
up from the mesh to the static root and records the first `StaticNode`
ancestor. When the built collider joins the world its handle maps to that
entity in `ColliderNodes` (a `HashMap<ColliderHandle, Entity>`, 16 bytes of
payload per collider). A rigid body's stand-in keeps its owner instead and
is not in the map. Names stay on the node entity (`Name`).

## Pick order

`pick_along` asks the fixed colliders first: a stand-in names its prop, a
mapped collider names its scene node, anything else is the level. The
nearest of that and the characters' spheres wins, as before.

## Pick-only geometry

`_null` groups are never drawn. The hub's shop counter is only a `_null`
box, so the pick must meet them; collision is untraced, so their colliders
carry `PICK_ONLY` and no interaction groups, and `exa_physics::solid`
filters them out of `fixed_hit`, `fixed_pick_except`, `level_sweep`, the
character controller's query pipeline and the bodies' level pieces. Only
`fixed_pick` (the hand's cursor pick) meets them. Their meshes hold
positions and indices only, with `RenderAssetUsages::MAIN_WORLD`, so they
cost no GPU memory and no tangent build at load.
