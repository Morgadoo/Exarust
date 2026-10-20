# Proposal

## Why

The creation screen switches between modes and toggles a pair of them by flag bits, refreshing the view. Exarust has the creation view routines but not the mode switch or toggle.

## What Changes

- Mode 2 stores the object as the global and calls its routine.
- Any other mode looks up the widget and clears bit `0x10` for a positive mode, sets it otherwise.
- Toggling leaves mode 2 with flag bit 1 and enters mode 2 with flag bit 2, refreshing; otherwise nothing changes.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the mode switch and toggle.

## Impact

- `exa_game::create`, from `switch_creation_mode` and `toggle_creation_mode`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` stays about 23% (about 23.3%).
- Evidence: the 74-byte `0x10012e930`: the mode 2 branch storing `0x158` at the global and calling `0x100111670`, the widget lookup `0x4534` and the `0x10` bit at `0x1c`. The 111-byte `0x10012f360`: the mode `0x274`, the flag bits 1 and 2 at `0x1004f0400`, the new modes 1 and 2 and the refresh `0x10012f950`. No new corpus files.
- Out of scope: the calls and lookup.

## Phase

Phase 5, character creation. The mode switch and toggle.
