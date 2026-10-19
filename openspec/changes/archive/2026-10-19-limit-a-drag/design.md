# Design

## Context

A drag looks along the cursor direction and keeps the nearest accepted hit. Separately, the grabbed box's eight corners are tested after they are placed.

## Goals / Non-Goals

**Goals:**

- Place the pull at the nearest hit inside a 1000 cm reach.
- Refuse a box with a corner outside −1000 cm to 280 cm.

**Non-Goals:**

- Testing the hit against level polygons, and pulling a door by its handle.

## Decisions

1. The reach starts at 1000 cm. Only a finite hit strictly closer replaces it.
2. The eight corners are the centre plus or minus each half-extent, then rotated and translated. The test reads the resulting height.
3. The comparisons are strict: 280 cm and −1000 cm are still allowed.

## Risks / Trade-offs

- A hit the polygon test would have rejected is still accepted here. The caller passes hits that test already kept.

## Migration Plan

No data migration.

## Open Questions

- None for this slice.
