# Proposal

## Why

Outdoor terrain showed grass only as texture. The original grows grass tufts
and plants from the palette's mask pass and a seed stored per tile.

## What Changes

- `exa_formats::terrain`: the tile's vegetation seed (formerly unknown).
- `exa_world::terrain::vegetation`: the original's point set, chunks, seeded
  choice of grass or plant, size and rotation, and ground placement. The
  plant variants are baked into one mesh per 6 m chunk.
- The density comes from the mask pass (materials' f13) layered per vertex.
- Tile skirts hide cracks between tiles.

## Capabilities

### New Capabilities

### Modified Capabilities
- `terrain`: vegetation.

## Impact

`crates/exa_formats`, `crates/exa_world`, docs.
