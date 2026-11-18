## Why

Roadmap task B4, lane `worlddraw2`: the port queue's lane Q17 (world draw:
grids, sectors, grass, weather) and the unclear re-audit's port items for
the light flood, the terrain manager and the static terrain. A re-flooded
light cleared every cell it had touched, where the original clears only a
box round its new cell; the terrain's GI block was picked per 6 m block,
where the original's terrain draw picks it once per tile; the live
composite pools ignored the game's texture-detail setting; and moving
roots and lights took the room of their cell clamped to the grid's edge on
every move, where the scene's per-frame cell update clamps one cell in and
runs the room hook only when the cell bytes change.

## Evidence

REA (Exanima.exe 0.9.5): the light flood's cleanup loop (box from the
light's cell, its radius over 600 cm through the round-up conversion helper
plus one, clamped to the grid, rows outer, one swap-remove per cell) and
the cell insert (no special case for the whole-cell window; the cell hook
runs for a widened entry and a new one); the static terrain draw (the GI
lookup runs once per tile, from the tile record's position, before its
blocks are drawn); the room manager's node hook (room from the node's cell
bytes, run only when the room's sector differs).

## What changes

- `exa_world::light_flood::CellLights::flood` clears only the box round the
  light's new cell; `cells_of` gives the cells its last flood recorded.
- `terrain.wesl` samples one GI block per tile; the live composite pools take
  the texture-detail setting.
- `exa_world::rooms::Rooms::cell_of_moving_node`; `scene_live` moves a root
  or light to another sector only when its cell bytes change.

## Impact

Lights that move far keep stale cell entries (as in the original). Terrain
blocks near a tile's edge may read another GI block than before (outdoor
levels with GI). No change at the default texture detail.
