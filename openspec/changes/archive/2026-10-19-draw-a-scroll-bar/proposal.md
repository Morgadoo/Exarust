# Proposal

## Why

The flat slider is in, and the same draw still owes the scroll bar: a trough and twelve quads.

## What Changes

- Mode 0x10000000 draws a trough inset by the unit, then twelve quads.
- The thumb sits at x plus the spare width times the value, ties rounded even. A second thumb sits three units further.
- Any other high mode draws neither the flat quads nor the scroll bar.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the scroll bar drawn by the skin, and the flat slider no longer claims that bar is absent.

## Impact

- `exa_ui::skin`, from `draw_flat_slider` when the mode is a scroll bar.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the trough and the twelve quads. No new corpus files.
- Out of scope: the frame, the centred icon, and the widget dispatcher.

## Phase

Phase 7, the interface. The scroll bar, not the frame.
