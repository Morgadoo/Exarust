# Proposal

## Why

The parent skin has a second slider-shaped method beside the 639-byte one: a track one pixel in from the left and two from the top, plus the same translated-thumb and filled-bar arms. Exarust models only the first.

## What Changes

- The plain arm draws a track from x + 1, y + 2 with width w − 2 and height h − 4, then the thumb at y + 1 + the inset.
- The thumb offset is the rounded product of the track width less twice the inset and less the thumb size, times the fraction.
- The filled arm fills the inset rectangle and draws a unit-wide bar at its own rounded offset.
- Any other style draws nothing.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the parent skin's second slider.

## Impact

- `exa_ui::skin`, from `draw_skin_scroll`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 588-byte parent method's two arms and its track offset. No new corpus files.
- Out of scope: the helpers' bodies.

## Phase

Phase 7, the interface. The second slider's layout.
