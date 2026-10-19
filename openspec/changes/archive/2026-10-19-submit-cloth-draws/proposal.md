# Proposal

## Why

Phase 3 lists the two cloth cull passes. It does not turn those passes into materials the renderer submits.

## What Changes

- A cloth reports the GPU face each pass culls: the front, then the back. A single-sided cloth reports only the back.
- A base material becomes one material per pass, and that material's pipeline key culls the named face.
- The world joins the two: the cloth's passes become those materials.

## Capabilities

### New Capabilities

### Modified Capabilities

- `cloth-step`: GPU submission of the two cull passes.

## Impact

- `exa_physics::cloth`, `exa_render` materials, `exa_world::cloth_draw`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the draw hooks set the two face culls, front then back, and draw each pass. No format field changes, so the corpus step is a skip.
- Out of scope: a custom render node, spawning a mesh for every cloth marker, body-to-body contact, and voxelising.
