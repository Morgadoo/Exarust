# Design

## Context

A follow replan asks for a path. The planner follows that path only when it is long enough, or when the search was forced.

## Goals / Non-Goals

**Goals:**

- Refuse a path of fewer than two points.
- Follow a forced path of two or more points.
- Follow an unforced path only when its ends are more than 150 cm apart.

**Non-Goals:**

- The grid search. It already returns the points.
- Steering along those points.

## Decisions

1. The length is the straight distance from the first point to the last, in centimetres. Exactly 150 cm does not follow.
2. Forced does not waive the two-point rule.

## Risks / Trade-offs

- The planner is not yet called from a placed character.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The accept rule was read before this change.
