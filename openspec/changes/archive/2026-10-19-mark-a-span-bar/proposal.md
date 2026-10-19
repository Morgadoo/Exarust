# Proposal

## Why

A span bar places its corners and stores the colour word, then skips the mark that bit 4 of the flag stores.

## What Changes

- Bit 4 stores the mark word once per corner, at the cursor, and the cursor then advances by one.
- Cursor 3 and mark 9 use indexes 3 through 14 and finish at 15.
- Flag 5, and a flag with the high bit and bit 4 set, still store them.
- Flag 0 and flag 1 store none, and the cursor still finishes 12 ahead.
- Cursor 4294967294 stores 4294967294, 4294967295 and 0, then finishes at 10.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the bit-4 mark on a span bar.

## Impact

- `exa_ui::skin`, from `draw_span_bar`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the flag test in the vertex writer. The flag and the mark word are unset in the image, so a fresh read is 0. No new corpus files.
- Out of scope: the other methods of the line class.

## Phase

Phase 7, the interface. The bit-4 mark, not the rest of the line class.
