# Proposal

## Why

The line class still has no span bar. The form dispatcher returns without drawing one.

## What Changes

- A span bar draws three quads and counts one preparation.
- Stored word 4 doubles to thickness 8. Stored word -3 uses thickness -6.
- At unit 5, origin 10, 20 and span 100, the middle, left cap and right cap take the corners below.
- The middle sample is bound once, then the cap sample twice.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the span bar on the line class.

## Impact

- `exa_ui::skin`, from `draw_span_bar`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the line-class method that follows the form dispatcher, its sample records, and the stored word it doubles. No new corpus files.
- Out of scope: the view scale, the colour word, the preparation body's material switch, and the other methods of the class.

## Phase

Phase 7, the interface. The span bar, not the rest of the line class.
