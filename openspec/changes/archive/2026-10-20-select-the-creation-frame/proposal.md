# Proposal

## Why

The creation screen follows a requested frame: it sets the frame one lower, refreshes and stores the part search result. Exarust has the search and several creation behaviours but not the frame selection.

## What Changes

- A requested frame below 1 does nothing.
- A current frame already equal to the requested frame less one does nothing.
- Otherwise the current frame becomes the requested frame less one, the part result is stored and the refresh runs.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the frame selection.

## Impact

- `exa_game::create`, from `select_creation_frame`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 12% to 13%.
- Evidence: the 128-byte frame select: the requested frame, the early return below 1 and on equality with the child's frame, the store of that frame, the model refresh, the part search and the store of its result. No new corpus files.
- Out of scope: the refresh and search bodies.

## Phase

Phase 5, character creation. The frame selection.
