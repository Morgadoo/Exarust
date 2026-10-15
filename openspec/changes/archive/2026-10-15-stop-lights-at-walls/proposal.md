## Why

Light and shadow casters still passed through walls: a light linked every
mesh within its radius, including the next room's. In exanima01 many lights
have a radius of 15 m, so each light's shadow map took in several rooms of
casters, and the light lit rooms it cannot see in the original. The original
floods each light through the tile grid's walls and links a mesh only if
the light sees it through that flood.

## What Changes

- The original's light flood for tile-grid levels: an angular window
  carried from cell to cell, narrowed by wall stubs, corner flags and doors
  and closed by solid walls. It is depth-first with the 31-entry stack
  limit, reach-tested in cells, and leaves a cell only through sides facing
  away from the light. Each cell records at most 12 lights, widening a
  light's window on re-entry.
- The node test: a ranged light links to a mesh only if the mesh's angular
  extent overlaps the window in its cell or a neighbouring cell its box
  spills into past no wall. A mesh around the light always passes.
- Lights are flooded in creation order when added or moved. Terrain tiles
  skip the test.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `rendering`: lights stop at walls.

## Impact

- `exa_world` (`light_flood.rs`, `shadow_links.rs`, `lib.rs`, `terrain`),
  `docs/RENDERING.md`.
