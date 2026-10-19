# Design

## Context

Equipping changes what is worn. Nothing places an icon in a pane.

## Goals / Non-Goals

**Goals:**

- Drop, move, refuse, remove and hit an icon by the recorded pane rules.

**Non-Goals:**

- Drawing the icon, testing its alpha, and the description card.

## Decisions

1. The icon is a rectangle of the caller's extent. The opaque-pixel test needs the rendered icon, which this pane does not have.
2. Capacity is the sum of the bulks present. A linked drop skips the check and can leave the pane over capacity.
3. The last drop is the top-most icon.

## Risks / Trade-offs

- The window chrome is not drawn. The functions are what the window would call.

## Migration Plan

No data migration.

## Open Questions

None for this slice.
