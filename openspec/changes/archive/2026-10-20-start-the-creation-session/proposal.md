# Proposal

## Why

The creation screen selects a table entry, starts a session by storing its value and scheduling, and dispatches its menu through an alternate or primary call. Exarust has the form and page routines but not these.

## What Changes

- The table selection stores the source index less one and applies the entry.
- Starting the session stores the value, clears the stored byte and schedules.
- The menu dispatch takes the alternate call when the global byte is set, else the primary call.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the table selection, session start and menu dispatch.

## Impact

- `exa_game::create`, from `select_creation_table`, `start_creation_session` and `dispatch_creation_menu`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 28% to 29%.
- Evidence: the 76-byte `0x10012e560`: the source `0x38` less one, the table `0x4f0560` and the call `0x100036440`. The 59-byte `0x10012ed60`: the value `0x4f0b80`, the byte `0x4ef330` and the schedule `0x1001246d0`. The 60-byte `0x10012ee60`: the byte `0x49a300` and the calls `0x10022d3c0` and `0x100029aa0`. The 65-byte `0x10012fcd0` repeats the shipped key handler. No new corpus files.
- Out of scope: the called bodies.

## Phase

Phase 5, character creation. The table selection, session start and menu dispatch.
