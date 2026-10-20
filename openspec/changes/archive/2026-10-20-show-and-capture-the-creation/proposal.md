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
- Evidence: the 64-byte `0x10012f5c0`: the stored byte `0x4ef330` and the mode `0x4effa0`. The 65-byte `0x10012f790`: the selection `0x6c`, the base call `0x10002dbc0` and the `0x40` result with slot `0xc8`. The 75-byte `0x10012f8e0`: the calls `0x100139430`, `0x1001316d0`, `0x10012f840`, the check `0x10017e740` and the build `0x10012f7e0`. The 123-byte `0x10012f2e0`: six qwords from `0x158 + 0xc0`, the `0x258` value, the `0x274` mode and the schedule `0x1001246d0`. No new corpus files.
- Out of scope: the called bodies.

## Phase

Phase 5, character creation. The form close, key, show and capture.
