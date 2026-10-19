# Proposal

## Why

Phase 3 decodes the cloth marker and does not step a cloth. `docs/COMPONENT-MAP.md` still lists cloth as missing.

## What Changes

- A particle keeps 0.98977 × 0.5^(1/83) of its velocity, dragged 5% toward the previous mean, then gains the body's gravity in centimetres.
- Wind above 0.001 moves it by direction · (noise + 0.25) · wind.
- A pin moves toward its rest point by its weight.
- One edge pass corrects by (L² / (|d|² + L²) − 0.5) · 0.8774.

## Capabilities

### New Capabilities

- `cloth-step`: one tick of a cloth.

### Modified Capabilities

## Impact

- `exa_physics::cloth`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the cloth step notes (retention, drag, wind, pins, the square-root-free edge correction). No format field changes, so the corpus step is a skip.
- Out of scope: collisions with bodies and the level, sleep, rustle, and drawing the mesh.
