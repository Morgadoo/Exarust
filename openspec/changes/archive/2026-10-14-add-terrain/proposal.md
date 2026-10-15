# Proposal

## Why

Outdoor arenas and parts of the story levels stand on terrain (`.rft` with a
`.ftb` palette) rather than the tile grid. Without it those levels float over
nothing (Phase 2 of `docs/PLAN.md`).

## What Changes

- `exa_formats::terrain`: terrain tiles (heights, hole mask, painted layers)
  and palettes (materials, blend rules, plants).
- `exa_world::terrain`: a mesh per tile, the original's blend-rule
  evaluation (height, slope, generated noise maps) into material weights,
  and a terrain material layering a tile's four heaviest materials with
  their blend masks; `spawn_level` spawns the terrain beside a level.
- `exa-tool corpus terrain`: parses every terrain and palette and reports
  unresolved layer names.
- `docs/formats/TERRAIN.md`.

Out of scope: terrain normal and gloss maps, mesh displacement from the
height pass, grass from the plant records, outdoor sun and sky, more than
four materials per tile.

## Capabilities

### New Capabilities
- `terrain`: decoding and showing the original terrains.

### Modified Capabilities

## Impact

`crates/exa_formats`, `crates/exa_world`, `crates/exa_tool`,
`crates/exarust` (log line), docs.
