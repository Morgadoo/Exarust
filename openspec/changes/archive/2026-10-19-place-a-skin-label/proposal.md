# Proposal

## Why

The parent skin has its own label placement. It shares the alignment helper with the line label but adds a drop-shadow arm when bit 0x40 is set.

## What Changes

- The alignment is the same as the line label's: style bits 0x7000 place x and 0x700 place y.
- Bit 0x10 selects the first ink.
- Mode 0x10000 selects the coloured renderer, with a one-pixel nudge on bit 0x40.
- Otherwise bit 0x40 draws two calls: a shadow one pixel left in the second ink, then one pixel down in the third ink.
- Any other word draws once in the fourth ink.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the parent skin's label placement.

## Impact

- `exa_ui::skin`, from `place_skin_label`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the parent label method and its four ink reads. No new corpus files.
- Out of scope: the glyph renderer.

## Phase

Phase 7, the interface. The placement, not the glyphs.
