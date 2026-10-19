# Proposal

## Why

A span bar still passes raw corners onward. The view shift, the two scales, and the colour word are not applied.

## What Changes

- Each corner is placed as x plus the shift, times the scale, and as the shift minus y, times the other scale.
- All twelve corners store the colour word.
- Shifts 2 and 100, scales one half and 2, and colour 17 place the first corner at 8.5, 160.
- An unset view places that corner at positive zero and negative zero, and stores colour 0.
- The raw quads stay.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the view placement and colour word of a span bar.

## Impact

- `exa_ui::skin`, from `draw_span_bar`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the vertex writer a span bar calls. The view floats are unset in the image. The colour word starts at 0. No new corpus files.
- Out of scope: the mark written when bit 4 is set, and the preparation body's material switch.

## Phase

Phase 7, the interface. The view and the colour word, not the bit-4 mark.
