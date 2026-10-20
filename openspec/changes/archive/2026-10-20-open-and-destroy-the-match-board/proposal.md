# Proposal

## Why

The arena match board has a show routine that seeds three globals and clears five panels, and a destructor that releases its list. Exarust has neither.

## What Changes

- Showing clears the stored pointer and byte, stores the seed and each of the three columns' child count less one, clears each of the five panels with a positive count, clears a present pointer and returns the base result; a present global gets the minus-one call.
- Destroying runs the base destructor on positive flags, clears the stored pointer, releases the list at `0x1a0`, runs the base and frees on positive flags.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the match board's show and destroy.

## Impact

- `exa_ui::match_board`, from `show_match_board` and `destroy_match_board`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `TMatchBoard` moves from 0% to 3%.
- Evidence: the 379-byte `0x100195c00`: the seeded globals `0x1004f1b10`, `0x1004f1b20`, `0x1004f1b30` and `0x1004f1b40`, the five panels at `0x448`, `0x788`, `0x7f0`, `0x858` and `0x8c0`, each with the `0x100107b30` clear, the pointer at `0x728`, the base `0x10002c410` and the global call `0x100029aa0`. The 121-byte `0x100195d80`: the flags guard, the `0x1004f1af0` clear, the `0x1a0` list release and the free. No new corpus files.
- Out of scope: the panel, base and global routines.

## Phase

Phase 4, UI. The match board's show and destroy.
