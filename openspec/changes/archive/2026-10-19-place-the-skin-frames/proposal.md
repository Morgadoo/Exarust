# Proposal

## Why

The parent skin draws two independent textured frames around a place, chosen by the style word. Exarust does not model that selection.

## What Changes

- Style bits 0x30 select the first frame: 0x10 uses the 0x9c then 0xb8 texture, 0x20 the 0xb4 then 0x9c texture.
- Style bits 0xc0 select the second frame: 0x40 uses 0x9c then 0xb8, 0x80 uses 0xb4 then 0x9c.
- The first frame uses the place rectangle; the second insets it by one on every side.
- Both switches run independently, so a style with one bit from each group draws two frames.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the parent skin's textured frames.

## Impact

- `exa_ui::skin`, from `skin_frames`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 487-byte parent method's two switches and the frame helper's two binds. No new corpus files.
- Out of scope: the frame helper's pixel geometry, which is recorded as a gap.

## Phase

Phase 7, the interface. The selection, not the strip geometry.
