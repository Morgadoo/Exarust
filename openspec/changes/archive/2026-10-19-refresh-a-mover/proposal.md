# Proposal

## Why

Phase 6 plans around walls. A search still pays for the footprint of the body that is asking, and a floor a body has moved is not raised again.

## What Changes

- A plan lifts the standing footprint, searches, and stamps it again. That search costs less than the same search with the footprint still down, and the centre is 63 again afterwards.
- A queued rectangle merges when its padded union is strictly smaller than the two padded rectangles. Sixteen are kept.
- A refresh grows each side by three cells and raises floors from samples. A changed floor is written and the cost passes run again.
- A zero-extent box covers its cell and a neighbour, inside the margin of three.

## Capabilities

### New Capabilities

### Modified Capabilities

- `navigation-grid`: planning around a footprint, and refreshing a dirty rectangle.

## Impact

- `exa_world::nav`, read by the path search already shipped.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, `docs/NAVIGATION.md`.
- Evidence: the planner's lift and stamp, and the mover's rectangle and floor raise. No format field changes, so the corpus step is a skip.
- Out of scope: the scene-wide countdown, reading samples out of a scene graph, and the follow rule of 150 cm, which is already shipped.
