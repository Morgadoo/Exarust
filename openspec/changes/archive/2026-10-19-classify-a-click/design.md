# Design

## Context

The cursor already picks a shape. It does not name the object under it or the distance at which a click can use that object.

## Goals / Non-Goals

**Goals:**

- Classify the object the same way the click does.
- Say when the body turns first, and when the point is already within reach.

**Non-Goals:**

- Starting the drag, playing the creak, and opening a container.

## Decisions

1. A blocked hinge, a held crank, a prop under mass 0.08, and a heavy prop under strength 0.9 are nothing.
2. A character answer replaces an item answer. A missing body also replaces it with nothing.
3. Reach uses the point's height, the vertical gap and the horizontal distance. A door uses the wider caps.

## Risks / Trade-offs

- The mass and strength checks apply only to a loose prop. A character is taken whatever it weighs.

## Migration Plan

No data migration.

## Open Questions

- None for this slice.
