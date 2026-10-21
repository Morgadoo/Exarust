# Proposal

## Why

The creation screen starts by resetting, creating and preparing the form, then sets its mode in mode 12 and stores it. Its apply step relinks, refreshes the camera, stores the part search and ticks. Exarust has the form close and session start but not the startup or step.

## What Changes

- Starting resets, resets again when the condition holds, creates the form when absent and prepares it.
- In mode 12 the form mode is 2 with the arena flag and 1 otherwise; the screen refreshes, builds when checked and ticks.
- The form and the value 2 are stored.
- The step relinks, refreshes the camera, stores the part result when a target exists and runs both ticks.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the startup and apply step.

## Impact

- `exa_game::create`, from `start_creation_screen` and `apply_creation_step`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 29% to 30%.
- Evidence: the 328-byte startup: the sentinel and reset calls, the form global, the prepare calls, the mode global against 12, the form's mode from the global threshold word, the refresh and build, the tick, and the two stores. The 129-byte step: its two refresh calls, the part search and the store of its result, and the two ticks. No new corpus files.
- Out of scope: the called bodies.

## Phase

Phase 5, character creation. The startup and apply step.
