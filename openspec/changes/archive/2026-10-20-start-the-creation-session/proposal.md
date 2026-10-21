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
- Evidence: the 76-byte table selection: the source less one, the global record table and the loader call. The 59-byte session start: the start value, the stored byte and the schedule. The 60-byte menu dispatch: the resume flag byte and its two calls. The 65-byte key handler repeats the shipped one. No new corpus files.
- Out of scope: the called bodies.

## Phase

Phase 5, character creation. The table selection, session start and menu dispatch.
