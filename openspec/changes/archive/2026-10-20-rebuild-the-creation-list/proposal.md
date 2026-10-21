# Proposal

## Why

The creation screen rebuilds its option list for an index: it builds the list, replaces any old one, captions a widget and draws. Exarust has the label refresh but not the list rebuild.

## What Changes

- A new list with no entries fails the rebuild.
- Otherwise the index is stored, an existing list is released, the new list is stored and the child's index follows.
- A present caption widget gets the byte `index + 0x41`.
- The list is drawn unless drawing is already suspended.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the option list rebuild.

## Impact

- `exa_game::create`, from `rebuild_creation_list`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 14% to 16%.
- Evidence: the 355-byte list rebuild: the label build, the list constructor and its count, the index store, the release, the child's record word, the widget 7 caption with `0x41`, the busy draw guard and the draw. No new corpus files.
- Out of scope: the list constructor, release and draw.

## Phase

Phase 5, character creation. The option list rebuild.
