# Design

## Context

Room tracking loads every door neighbour as adjacent. There is no doorway test and no passage.

## Goals / Non-Goals

**Goals:**

- A shut door leaves the far room hidden.
- Sight through an opening, and not beside it or through a shut door.
- A passage replaces the loaded level name.

**Non-Goals:**

- The GPU clip of the adjacent room's mesh, and light through the portal.

## Decisions

1. Doors not listed are open, so today's rooms stay as they are.
2. The doorway is a vertical rectangle. The segment from the eye to the point must cross it.
3. `enter_level` looks the destination id up in the caller's level list.

## Risks / Trade-offs

- The mesh of an adjacent room is still drawn in full when the room is loaded. The clip decides which points count as seen. The GPU scissors come later.

## Migration Plan

No data migration.

## Open Questions

None.
