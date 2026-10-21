# Proposal

## Why

The creation screen closes a form by mode, handles a key with or without a selection, shows a form through four calls, and captures six words plus a value and a mode. Exarust has the close, mode and hit routines but not these.

## What Changes

- Closing the form clears the stored byte and schedules the special form in mode 10, otherwise the regular form.
- A key with a selection forwards and returns the base result; without one it returns `0x40` and closes.
- Showing prepares, dispatches, refreshes and builds when the check passes.
- Capturing copies six words, computes `current − last + saved`, stores the mode and schedules.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the form close, key, show and capture.

## Impact

- `exa_game::create`, from `close_creation_form`, `creation_key`, `show_creation_form` and `capture_creation`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 26% to 27%.
- Evidence: the 64-byte show: the stored byte and the mode global. The 65-byte key: the selection, the base call and the `0x40` result with the close slot. The 75-byte setup: its three calls, the conversion check and the build. The 123-byte capture: six qwords from the slot table, the index value, the mode and the schedule. No new corpus files.
- Out of scope: the called bodies.

## Phase

Phase 5, character creation. The form close, key, show and capture.
