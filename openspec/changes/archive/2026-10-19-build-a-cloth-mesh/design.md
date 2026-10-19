# Design

## Context

The tick can move particles. Nothing builds them from triangles, and the wind sample is a number the caller types in.

## Goals / Non-Goals

**Goals:**

- Build edges and both windings from a mesh, sample the noise tile, and report the collision reaction.

**Non-Goals:**

- The shortest-path pin distances in the original setup, and the actual draw call.

## Decisions

1. A bend link joins the two vertices opposite one shared edge.
2. The tile cell is 2 m. The walk matches the pre-step: inside the range, `(random − 0.5) · wind · 0.8`; outside, half that stride pushes the sample back.
3. The reaction is the negated sum of the collision displacements, not an average.

## Risks / Trade-offs

- The original setup also ranks vertices by distance from the pins. This slice does not.

## Migration Plan

No data migration.

## Open Questions

None for this slice.
