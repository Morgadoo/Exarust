# Proposal

## Why

Phase 3 steps a cloth that is already a list of particles. It does not build that list from a mesh, sample the wind tile, hand a reaction to the body, or draw both faces.

## What Changes

- A triangle mesh becomes one particle per vertex, a link on every edge, and a bend link between the vertices opposite a shared edge. Named vertices are pinned.
- The 16×16 wind tile walks each sample and is sampled bilinearly in 2 m cells.
- The sum of this tick's collision pushes is returned, negated, as the body's reaction.
- The face list contains both windings.

## Capabilities

### New Capabilities

### Modified Capabilities

- `cloth-step`: mesh setup, the noise tile, the body reaction and both faces.

## Impact

- `exa_physics::cloth`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the cloth setup and wind notes. No format field changes, so the corpus step is a skip.
- Out of scope: the geodesic distances used to order pins, and submitting the faces to the renderer.
