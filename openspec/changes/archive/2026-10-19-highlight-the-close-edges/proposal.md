# Proposal

## Why

The form dispatcher draws the close square, but not the three edge tints the hit test returns for it. The dispatcher checks the place's still bit and the close mark -9, -10 or -8 and paints an orange highlight with the tab sample.

## What Changes

- With the still bit set, close mark -9 draws a strip from x0 plus one unit to x0 plus three units between y0 and y0 plus two units.
- Mark -10 draws the same strip below y0.
- Mark -8 draws a four-unit square at x0, y0.
- x0 is the close x plus the rounded 0.15 of the unit. y0 is the close y plus three units when mask bits 1 and 4 are both set, else two units.
- The highlight uses the tab sample and the colour 255, 204, 153, 255.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the close-edge highlight on the form dispatcher.

## Impact

- `exa_ui::skin`, in `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the dispatcher's three close-mark branches and the colour helper. No new corpus files.
- Out of scope: the click path that produces the marks.

## Phase

Phase 7, the interface. The tint, not the click.
