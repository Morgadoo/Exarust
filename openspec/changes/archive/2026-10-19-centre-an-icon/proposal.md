# Proposal

## Why

The frame is in, and the same skin still owes the centred icon: where a glyph sits, and which ink a caption uses.

## What Changes

- A centred icon sits at the origin plus half the spare between the widget and the icon size.
- A marked state passes the scale bits of 1, and bit 0x40 inside the band adds one to the height.
- Any other state picks one of five inks. The glyph call is not in.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the centred icon drawn by the skin.

## Impact

- `exa_ui::skin`, from `draw_centred_icon`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the half-spare place, the nudge, and the ink choice. No new corpus files.
- Out of scope: the glyph call and the widget dispatcher.

## Phase

Phase 7, the interface. The centred icon, not the widget dispatcher.
