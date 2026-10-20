# Proposal

## Why

After the parallel solver, PostUpdate took 8.5 ms of play mode's frame, the
largest single part being the shadow links: the player's carried torch moves
every frame, and each move relinked all of exanima01's ~31 000 casters
against it. The original relinks a moved light only against the nodes of
the sectors it reaches.

## What Changes

- The shadow links keep every caster filed by the 6 m cell of its sphere's
  centre (a sphere wider than a cell is kept apart), updated when a caster
  is relinked.
- A moved light is relinked only against the casters in the cells its reach
  (plus a cell) touches, the wide ones and the casters already linked to
  it. The links that result are the same.

## Capabilities

### Modified Capabilities

- `rendering`: how a moved light finds the casters it may link.

## Impact

- `exa_world::shadow_links` (`CasterCells`).
- `docs/RENDERING.md` (Shadow links), `docs/PERFORMANCE.md`.
- Evidence: `context/re/optimizations.md` §4.1–§4.2 (lights registered per
  sector through the wall grid; links per node of those sectors).
- Out of scope: the per-frame face marks of characters' many skinned
  meshes (the original merges a character's skinned meshes into one).

## Phase

Phase 8, performance.
