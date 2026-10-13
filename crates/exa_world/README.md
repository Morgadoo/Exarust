# `exa_world`

Levels: tile-grid dungeons built from wall/floor sets, terrain levels, object placement, sectors/portals, doors and triggers.

**Phase:** Phase 2 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: skeleton.

## Scope

- Load level `.rfc` (grid header, wall/floor set references, placed objects) and companion `.rft` terrain.
- Instantiate modular pieces from `Resource.rpk` sets and `Objlib.rpk` props.
- Sectors, portals and visibility; level transitions; save-game world state.
