# Proposal

## Why

Phase 3 builds cloth edges and both windings. It does not record how far each vertex is from a pin along the mesh, and the double-sided draw is only a face list, not the two cull passes.

## What Changes

- Each vertex stores its distance along triangle edges from the nearest pin. With no pins, vertex 0 is the source. The distance follows the mesh, not the straight line.
- The draw hook emits two passes: cull the front and draw, then cull the back and draw. A single-sided cloth draws only its front.

## Capabilities

### New Capabilities

### Modified Capabilities

- `cloth-step`: geodesic pin distances and the two-pass cull.

## Impact

- `exa_physics::cloth`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the cloth setup walk and the draw hooks that set the two face culls. No format field changes, so the corpus step is a skip.
- Out of scope: submitting the passes to the GPU.
