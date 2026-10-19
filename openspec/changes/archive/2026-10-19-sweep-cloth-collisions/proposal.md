# Proposal

## Why

Phase 3 pushes a cloth vertex out of a resting sphere and an infinite plane. The original sweeps a body sphere through cloth triangles and sweeps each vertex through level triangles.

## What Changes

- A body sphere moving from one point to another that crosses a cloth triangle pushes the vertices it reaches.
- A sweep that misses the triangle leaves those vertices where they are.
- A vertex whose segment crosses a level triangle stops 2 cm short of the face it came from. A segment that misses the triangle is not moved.

## Capabilities

### New Capabilities

### Modified Capabilities

- `cloth-step`: swept sphere-triangle and vertex-triangle tests.

## Impact

- `exa_physics::cloth`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the cloth collision notes (a sphere swept between poses against cloth triangles; a per-vertex sweep against level triangles). No format field changes, so the corpus step is a skip.
- Out of scope: the impulse applied back to the body, and drawing the cloth.
