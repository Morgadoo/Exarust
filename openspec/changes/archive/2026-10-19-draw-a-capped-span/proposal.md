# Proposal

## Why

The line class still has no capped span after the bevel box. The next method draws two end caps and the middle bar between their inner edges from two anchors and a vertical centre.

## What Changes

- Unit 5 with left 10, right 110 and centre 28 draws a left cap from 0, 18 to 30, 38, a right cap from 120, 18 to 90, 38, and a middle from 30, 28 to 90, 38.
- The cap sample is 0.15, 0.2, 0.15, 0.4, then 0.3, 0.4, 0.3, 0.2, bound twice.
- The middle sample is the span bar's 0.3, 0.1, 0.3, 0.2, then 0.5, 0.2, 0.5, 0.1, bound once.
- One mode call with argument 2, one begin, one preparation, one ink bind, twelve colour writes and one end call follow.
- The second positional float is never read by the method, so it is not taken.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the capped span on the line class.

## Impact

- `exa_ui::skin`, from `draw_capped_span`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the line-class method at the slot after the bevel, its end-cap and middle samples, and its twelve corner calls. No new corpus files.
- Out of scope: the caller that supplies the anchors and the methods around it.

## Phase

Phase 7, the interface. The capped span, not its caller.
