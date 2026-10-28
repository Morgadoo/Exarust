# Tasks

- [x] Record each static collider's nearest scene node at build time
- [x] Return a scene node's mesh as `Hit::SceneNode`; keep every consumer's behaviour
- [x] Read the hub's clicked node name and side from the scene node
- [x] Build `_null` groups as bare pick-only meshes and colliders
- [x] Review: keep pick-only geometry out of collision and the gameplay rays; never upload it; skip skinned seams
- [x] Review: count the play view's practice stand-in as the arena while the hub is loaded
- [x] Tests: the node map, pick-only filtering, the hub clicks, arenahub's shop and notice board under a ray
