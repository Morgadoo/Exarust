# Proposal

## Why

The draw skin's constructor is in, and the flat slider is the next draw still missing from that 35 KB component.

## What Changes

- A slider whose high mode bits are clear draws two end caps, the track between them, and a thumb.
- The thumb sits at the GUI unit plus the spare width times the value, with ties rounded even.
- The state bits choose one of three inks and can count a mark. Any other mode draws no flat quads.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the flat slider drawn by the skin.

## Impact

- `exa_ui::skin`, from `draw_flat_slider`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the flat path's four quads, the three samples, and the rounded thumb. No new corpus files.
- Out of scope: the scroll bar, the frame, the centred icon, and the widget dispatcher.

## Phase

Phase 7, the interface. The flat slider, not the scroll bar.
