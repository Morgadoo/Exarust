# Proposal

## Why

The mind-sight overlay (`0x1001e01d0`) fades in and out over two seconds and the sight's target pick (`0x1001def00`) uses a scaled radius of `n·scale·0.5·40/width`. Exarust has the spirit form and the mind toggles but not the overlay phase or the pick radius.

## What Changes

- Idle draws nothing; shown forces alpha 1 and starts fading out past its end.
- The 2 s fades move alpha by the elapsed milliseconds; the fade-out ends below 0.001.
- The pick radius is `n·scale·0.5·40/width`, and the nearest candidate within its radius wins.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the mind-sight overlay and pick.

## Impact

- `exa_game::mind`, from `MindPhase`, `MindOverlay`, `advance_mind_overlay`, `mind_pick_radius` and `pick_mind_target`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Thaum_Mind` becomes **6%**. The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: `0x1001e01d0`: the phase at `+0x54`, the alpha at `+0x58`, the end at `+0x50`, the stamp at `+0x4c`, the 2000 ms double at `0x10032f2d0`, the 0.001 double at `0x10032f2c8` and the 1.0 clamp; `0x1001def00`: the `n·scale·0.5·40/w` radius (0.5 and 40.0 at `0x10032f1d0`/`0x10032f1c8`), the normalized distance and the `≤ 1` nearest test. No new corpus files.
- Out of scope: the billboard basis and the shader upload.

## Phase

Phase 7, thaumaturgy. The mind-sight overlay.
