# Design

## Context

The in-scene projection and the 100 cm carrot are shipped. The other branch, taken when the low two scene bits are clear, slides the body with no physics.

## Goals / Non-Goals

**Goals:**

- Advance 2 cm along the stored segment, or cross once when the remainder is not strictly longer than 2 cm.
- Raise only the standing height by the pelvis base.

**Non-Goals:**

- The run bit, the soft stop, and the line walk.
- Changing the in-scene carrot.

## Decisions

1. The slide runs only when `scene & 3 == 0`. Any other low bit leaves the slide unrun.
2. A horizontal length squared at or below the tiny epsilon uses the length 0.1. The step stays only when that length is strictly greater than fraction · length + 2.
3. The cross writes one new fraction, `(1 − (1 − fraction) · length) · (1 − fraction) − 2`, divided by the next horizontal length. It can be negative. The last segment stops at its end instead of reading past the path.

## Risks / Trade-offs

- A negative fraction places the body behind the corner it just crossed. That is the step the original takes.

## Migration Plan

No data migration. No format page: no field changes.

## Open Questions

- None for this slice. The branch and the 2 cm step were read before this change.
