## Why

RF_SceneNodes' gap: the scene-node model (`exa_world::scene_nodes::Scene`)
was built only by tests and the scene editor; the running game drew from
the room states alone. TRenderer's gap: the renderer's per-sector walk of
the blended lists was not reproduced (no per-object sector), so blended
surfaces in different rooms drew in one list.

## Evidence

REA on the renderer's frame: the scene pass walks the alpha-tested,
opaque, decal and blended lists each as an outer loop over the scene's
sectors in scene order (a sector skipped when its visibility word misses
the renderer's mask, its scissor set when it keeps a rectangle), then the
list's states in list order, then that sector's entries of each state
whose mesh word meets the mask. A second blended walk (the sector's own
word, the scissor test switched per entry on the `0x1000_0000` bit, the
state's per-entry hook, direct draws) is chosen by a word of an object the
scene holds; it walks the sectors in the same order. The words the walks
test are written by the scene unit's per-frame update (classification,
hidden and visible passes). The mesh preparation sets the option bit the
sector bounds split on when one of the mesh's parts' materials casts
shadows (the material byte the material loader sets from the same rule as
`casts_shadow`).

## What Changes

- exa_world: `scene_live` keeps a `Scene` for the loaded grid level (rooms
  as sectors with their door portals, every mesh registered, moves,
  removals, frames, bounds, the frame with the room states and the main
  view) and trims the main camera's visible list by the visible pass;
  `LevelRooms::portals`; `Scene::reuse_node` and `Scene::quiet_passes`.
- exa_render: `sector_walk` re-sorts the transparent phase sector-major for
  the decal and blended lists from `DrawSectors`.
- exarust: `--no-sector-cull`.
