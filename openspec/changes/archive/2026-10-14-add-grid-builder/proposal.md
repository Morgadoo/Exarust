# Proposal

## Why

Dungeon levels draw their walls, floors and ceilings from a tile grid and
modular sets; without them a level is props floating in the dark. This is
the remaining core of `docs/PLAN.md` Phase 2 (milestone M2, "walkable world").

## What Changes

- Index the pieces of wall sets and floor sets by name, as the original does.
- Build every open cell of a level's tile grid: centre floor, edge strips,
  walls by edge code and variant, corners, special tiles with their masks,
  optional ceilings.
- Decode material diffuse/specular colours so colour-only materials (wall
  caps, `_black`, `_dark`) look right; skip `_null` geometry.

Evidence: the grid builder in `Exanima.exe` (REA), screenshots of the arena
and the first story level.

Out of scope: the alternate builder for cells flagged `0x800000`, merging
geometry per room, the floor of special tiles beyond the centre piece,
terrain, collision.

## Capabilities

### New Capabilities
- `grid-building`: assembling dungeon walls, floors and ceilings from a level's tile grid and modular sets.

### Modified Capabilities

## Impact

`crates/exa_world` (`grid` module), `crates/exa_formats` (material colours),
`crates/exa_assets` (material colours, `_null`), `crates/exarust`
(`--ceilings`), `docs/formats/RFC.md`, `docs/PLAN.md`.
