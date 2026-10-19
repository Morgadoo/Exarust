# Proposal

## Why

The scroll bar is in, and the same skin still owes the frame: four corner squares and the edge bars.

## What Changes

- A frame draws four corners of three times the unit and two edge bars one unit thick.
- Side bars appear only when four times the unit is below the height.
- The state, the mark word, and the wide bit choose one of three inks.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the frame drawn by the skin.

## Impact

- `exa_ui::skin`, from `draw_frame`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the corner squares, the edge bars, and the ink choice. No new corpus files.
- Out of scope: the centred icon and the widget dispatcher.

## Phase

Phase 7, the interface. The frame, not the widget dispatcher.
