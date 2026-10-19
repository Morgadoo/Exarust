# Proposal

## Why

The parent skin draws a control's image centred in its place, choosing an ink from the style and flag words and an optional frame from the style's top nibble. Exarust does not model that draw.

## What Changes

- The centre is the place's position plus half the spare width and height around the image's square size; the marked band nudges y by one.
- Mode 0x10000 draws through the scaled image call with scale 1; every other mode binds an ink.
- With style bit 0x1000000, flag bit 0x10 binds the 0xd8 slot, flag bit 0x40 binds 0xf4 with the raised colour, and any other word binds 0xe4 with the plain colour.
- Without that style bit, the same slots are bound but without an explicit colour.
- Style bits 0x70000000 select a frame index in every arm.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the parent skin's image draw.

## Impact

- `exa_ui::skin`, from `draw_skin_image`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 788-byte parent method's centre arithmetic, four ink arms and frame nibble. No new corpus files.
- Out of scope: the image object and the two draw helpers' pixel bodies.

## Phase

Phase 7, the interface. The selection, not the image pixels.
