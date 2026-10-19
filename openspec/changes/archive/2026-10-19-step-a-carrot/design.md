# Design

## Context

`start_follow` already accepts a path. The point the walker aims at, 100 cm ahead of its projection, is not computed yet. The metre-space order already has its own carrot and is left alone.

## Goals / Non-Goals

**Goals:**

- Project a ground point onto a centimetre polyline and return the point 100 cm further along.

**Non-Goals:**

- Moving a body, writing a heading, or writing a velocity.
- The out-of-scene 2 cm slide, the run bit, and the line walk.
- Changing the 150 cm accept rule.

## Decisions

1. The stored fraction is the unclamped horizontal parameter. The projection used for distance clamps that parameter. The 100 cm is measured from the unclamped foot.
2. A strictly closer segment replaces the best. A tie keeps the earlier one. A segment whose horizontal length is below the tiny epsilon is skipped.
3. Exactly 100 cm of segment remaining spills forward. A step whose parameter is below the epsilon has no goal, and the cursor stays the projection.

## Risks / Trade-offs

- The goal is a point. The walker does not steer toward it yet.

## Migration Plan

No data migration. No format page: no field changes.

## Open Questions

- None for this slice. The projection and the 100 cm step were read before this change.
