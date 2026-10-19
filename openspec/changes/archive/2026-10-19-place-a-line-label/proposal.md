# Proposal

## Why

The line class still has no label draw after the capped span. The next method measures the text, aligns it in the place from the style bits, and calls the text renderer with an ink chosen by the state word.

## What Changes

- The measure is the sum of glyph advances; the line height and base come from the font.
- Horizontal alignment from style bits 0x7000: 0x1000 left of the place, 0x2000 centred, 0x3000 right inside, 0x4000 right outside, and any other word the place inset plus half a unit.
- Vertical alignment from style bits 0x700: 0x100 half a unit up, 0x200 centred with the line height, 0x300 at the base, 0x400 at the bottom, and any other word half a unit down.
- State bit 0x10 selects the first ink with no nudge. Mode 0x10000 selects the coloured renderer and nudges by one when bit 0x40 is set. Bits 0x60 select the third ink with a one-pixel nudge. Any other word selects the second ink with no nudge.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: label placement on the line class.

## Impact

- `exa_ui::skin`, from `place_line_label`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the line-class label method, its shared alignment helper, and the font table's widths, line height and base. No new corpus files.
- Out of scope: rasterising the text and the methods around it.

## Phase

Phase 7, the interface. The placement, not the glyph renderer.
