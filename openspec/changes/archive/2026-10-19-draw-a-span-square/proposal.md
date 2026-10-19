# Proposal

## Why

The line class still has no square after the span bar. The next method draws one square from a point and the stored word.

## What Changes

- Stored word 4 at 10, 20 draws (3, 29), (3, 13), (19, 13), (19, 29).
- Stored word 6 draws (-1, 33), (-1, 9), (23, 9), (23, 33).
- Stored word -1 places the first corner at bits 0xdf000000, 0xdf000000.
- The cursor starts at 0. Flag 4 and mark 9 store indexes 0 through 3 and finish at 4. Flag 0 stores none and still finishes at 4.
- Colour 17 is stored four times.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the square on the line class.

## Impact

- `exa_ui::skin`, from `draw_span_square`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the line-class method after the span bar, its sample, and the stored word shifted by two. No new corpus files.
- Out of scope: the scale rebuild inside the begin call, and the other line-class methods.

## Phase

Phase 7, the interface. The square, not the scale rebuild.
