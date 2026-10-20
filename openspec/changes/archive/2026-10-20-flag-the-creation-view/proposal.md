# Proposal

## Why

The creation view refreshes by four calls and sets or clears the `0x10000` flag from the option word, then stores the part search. Exarust has the tick and size but not these.

## What Changes

- Refreshing runs the link, camera, first and second ticks.
- The flag sets bit `0x10000` when set and clears it otherwise, then refreshes and stores the part result.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the view refresh and flag.

## Impact

- `exa_game::create`, from `refresh_creation_view` and `set_creation_flag`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 20% to 21%.
- Evidence: the 65-byte `0x10012cbc0`: the four calls `0x1000f13c0`, `0x100133020`, `0x100071a20` and `0x1001e5ea0`. The 108-byte `0x10012d320`: the `0x11` option word, bit `0x10000` on `0x54`, the refresh `0x1000ef7f0`, the part search `0x10012cb10` and the store at `0x180 + 0x180`. No new corpus files.
- Out of scope: the four refresh calls and the part search body.

## Phase

Phase 5, character creation. The view refresh and flag.
