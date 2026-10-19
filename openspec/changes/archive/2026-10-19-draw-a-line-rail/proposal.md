# Proposal

## Why

The line class still has no rail after the label. The next method draws a track along the bottom of a place from two end caps and a middle, then a thumb at the right edge.

## What Changes

- Unit 5 at place 10, 20 of size 100 by 80 draws a left cap from 0, 85 to 30, 105, a right cap from 100, 85 to 90, 105, and a middle from 30, 95 to 90, 105.
- The thumb runs from 105, 30 to 95, 40.
- The cap sample is the capped span's 0.15, 0.2, 0.15, 0.4, then 0.3, 0.4, 0.3, 0.2, bound twice. The middle is the span bar's, bound once. The thumb sample is 0.5, 0.8, 0.5, 0.9, then 0.55, 0.9, 0.55, 0.8.
- Bit 0x10 selects the first ink, otherwise the second. Mode 0x10000 rebinds the thumb in the third ink with alpha 255.
- One mode call with argument 2, one begin, one preparation, four binds, sixteen colour writes and one end call follow.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the rail on the line class.

## Impact

- `exa_ui::skin`, from `draw_line_rail`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the line-class rail method, its three samples and its corner arithmetic. No new corpus files.
- Out of scope: the value that positions the thumb and the methods around it.

## Phase

Phase 7, the interface. The rail, not its value.
