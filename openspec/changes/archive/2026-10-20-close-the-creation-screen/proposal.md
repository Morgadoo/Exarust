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
- Evidence: the 442-byte `0x10012f3d0`: the active word `0x270`, the mode `0x4effa0`, the guarded fields `0x208`, `0x220`, `0x218`, `0x238`, `0x210`, `0x228` and `0x1d0`, the `0x2d` call, the `0x28c`/`0x2f4` resets, the tick `0x10002e760`, the strings `0x1003161b0`, `0x1003161d0` and `0x1003161f0`, and the `0x10` bit. No new corpus files.
- Out of scope: the release, tick and string bodies.

## Phase

Phase 5, character creation. The close.
