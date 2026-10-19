# Proposal

## Why

Phase 3 steps a cloth particle and does not yet push it out of a body or the level, sleep it, rustle, or hand vertices to a mesh.

## What Changes

- A vertex inside a body sphere is pushed out to the radius plus 2 cm. Tangential motion is scaled by one minus the body's friction, one of 0.002, 0.01 and 0.015.
- A vertex behind a level plane is pushed out to 2 cm along the normal.
- Sleep counts down while the mean squared speed is under √(vertices × 10⁻⁶), and never below the owner's count. Asleep cloth does not fall. A push wakes it.
- Rustle plays when the squared sum of the velocities exceeds 400. Its envelope decays by 0.9.
- The mesh vertices are the particle positions.

## Capabilities

### New Capabilities

### Modified Capabilities

- `cloth-step`: collisions, sleep, rustle and the mesh.

## Impact

- `exa_physics::cloth`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the cloth step notes. No format field changes, so the corpus step is a skip.
- Out of scope: swept tests against cloth triangles, and which body uses which of the three frictions.
