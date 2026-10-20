# Proposal

## Why

The power tree's hover handler validates the event kind, hit-tests the mouse point, updates the tooltip entry and sound, and restores the defaults on a miss. Exarust does not model it.

## What Changes

- An event other than kind 0xae56ca01 is refused.
- A positive node index copies the previous entry when set, merges the event word into the entry's 0x700000 bits, stores the event's 0x20 word and plays the hover sound when ready, returning true.
- A non-positive index copies the defaults when a previous entry is set and returns false.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the power tree's hover.

## Impact

- `exa_game::thaum`, from `hover_tree`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 384-byte hover method's kind guard, hit index, entry copy and sound. No new corpus files.
- Out of scope: the hit test and the sound.

## Phase

Phase 5, thaumaturgy. The tree's hover.
