# Proposal

## Why

The creation screen hit-tests its option list for a key, selecting the last entry on a hit, and lazily creates its singleton. Exarust has the list rebuilds and steps but not the hit or the singleton.

## What Changes

- Key 8 or a missing list never hits; otherwise the hit result decides.
- A hit selects the last entry, and the result is true either way.
- The singleton is created when absent and returned otherwise.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the option hit and the singleton.

## Impact

- `exa_game::create`, from `hit_creation_option` and `ensure_creation_singleton`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 25% to 26%.
- Evidence: the 80-byte `0x1001315d0`: the key 8 early return, the list `0x230`, the hit `0x100108430` and the selection `0x100108750` at the count less one. The 76-byte `0x100130490`: the singleton `0x1004f0450`, its creator `0x100130290` and the hook at `0x10049a9d0`. No new corpus files.
- Out of scope: the hit, selection and creator bodies.

## Phase

Phase 5, character creation. The option hit and the singleton.
