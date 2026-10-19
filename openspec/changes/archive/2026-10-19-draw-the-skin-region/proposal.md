# Proposal

## Why

The last unmodelled `TGUIDrawSkin` owner is a 965-byte scroll region: it translates, clips, computes scroll offsets, replays buffered quads, visits visible children and adds a scrollbar detail. Exarust has no counterpart.

## What Changes

- Mode bit 0 skips the region after the first begin.
- A failed clip intersection also stops after the first begin.
- Otherwise the clip is the origin plus the inset with the extent less the inset, and the scroll offsets are the rounded products of the totals less those spans and the stored fractions, or zero on flag 0x800.
- Every buffered quad is replayed and every visible child is visited.
- Flag 0x80 adds a 0x30 detail at the scroll offsets; flag 0x100000 adds a 0x50 detail with four units; the offsets are the unit and four units.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the parent skin's scroll region.

## Impact

- `exa_ui::skin`, from `draw_skin_region`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 965-byte parent method's clip, scroll, quad loop and child loop. No new corpus files.
- Out of scope: the global clip object and the scissor calls.

## Phase

Phase 7, the interface. The region decision, not the GPU scissor.
