# Proposal

## Why

Closing the creation screen either schedules a form or releases a list of guarded children, resets widget words, ticks and writes strings. Exarust has the opening and mode routines but not the close.

## What Changes

- An inactive child clears the stored byte and schedules the special form in mode 10, otherwise the regular form.
- An active child releases the first present of `0x208`, `0x220` (also calling `0x2d`) and `0x218`, then `0x238` and `0x210`.
- Outside mode 12 it releases `0x228` and `0x1d0` and resets two widget words.
- It ticks, clears the active word, writes three strings and clears the widget bit.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the close.

## Impact

- `exa_game::create`, from `close_creation_screen`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 23% to 25%.
- Evidence: the 442-byte close: the active word, the global mode, the seven guarded objects, the `0x2d` call, the two widget resets, the tick, the three strings, and the `0x10` bit. No new corpus files.
- Out of scope: the release, tick and string bodies.

## Phase

Phase 5, character creation. The close.
