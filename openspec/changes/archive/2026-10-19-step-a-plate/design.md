# Design

## Context

A lever already creaks and signals. A pressure plate is the other operative: several travels, a grind, and a click.

## Goals / Non-Goals

**Goals:**

- Raise or lower every travel, then write their mean back.
- Grind from the change in that mean.
- Click and signal when the mean crosses 0.7, and release below 0.5.

**Non-Goals:**

- A body standing on the plate, and the click's random pitch.

## Decisions

1. A one-shot clears the press as soon as one travel passes 1. Later travels in that same tick fall.
2. The grind stops when the speed is not finite or is at most 0.001.
3. Press is a mean strictly above 0.7, and only from the up state. Release is a mean strictly below 0.5, and only from the down state. Inverted plates still click down, and they do not signal on the way up.
4. An empty plate does not change.

## Risks / Trade-offs

- A placed plate does not yet call this each tick. The function is the step's rule. `pushed` is an input.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The step was read before this change.
