# Design

## Context

The cloth marker is decoded. No particle moves.

## Goals / Non-Goals

**Goals:**

- One tick a test can run: fall, wind, pin, and an edge that shortens when stretched and stays when it is at its length.

**Non-Goals:**

- Sphere and triangle collisions, sleep, the rustle sound, and the render mesh.

## Decisions

1. Positions are centimetres. Gravity is the body's −9.80665 m/s², converted per tick.
2. The shipped cloth mode takes one constraint pass, so the step does one.
3. Wind noise is an input. The random walk reflects a sample into −1 to 1.

## Risks / Trade-offs

- The step is not yet called from the character tick. The function is what that tick would call.

## Migration Plan

No data migration.

## Open Questions

None for this slice.
