# Proposal

## Why

The parent skin draws a slider in two styles: a plain track with a translated thumb, and a filled track with a unit-wide bar. Exarust does not model either.

## What Changes

- Style 0 draws a track inset by two on every side, translates by the thumb offset, draws the thumb, and translates back.
- The thumb offset is the rounded product of the track width less twice the inset and less the thumb size, times the stored fraction.
- Style 0x10000000 fills an inset rectangle with the stored paint pointer, then draws a unit-wide bar at the same offset.
- Any other style draws nothing.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the parent skin's slider.

## Impact

- `exa_ui::skin`, from `draw_skin_slider`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 639-byte parent method's two style arms and its four helpers. No new corpus files.
- Out of scope: the paint pointer's contents and the helpers' bodies.

## Phase

Phase 7, the interface. The two styles, not their pixels.
