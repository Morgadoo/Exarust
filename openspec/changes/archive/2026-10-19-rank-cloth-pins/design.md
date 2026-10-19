# Design

## Context

`from_mesh` makes particles, edges and both index windings. Pins are a weight of 1 at the pinned point. Nothing ranks the other vertices.

## Goals / Non-Goals

**Goals:**

- Store the mesh distance from the pins, and list the two cull passes.

**Non-Goals:**

- Issuing those passes to the renderer.

## Decisions

1. Distance is Dijkstra on triangle edges, weighted by their length. A missing pin list starts at vertex 0, as the setup does.
2. The two passes cull the front, then the back. That is the double-sided hook. One-sided cloth keeps only the back cull.

## Risks / Trade-offs

- The original also threads that distance through its edge builder. Here the distance is stored beside the edges.

## Migration Plan

No data migration.

## Open Questions

None for this slice.
