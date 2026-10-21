# Proposal

## Why

The creation screen refreshes its part result through a target and destroys itself by decrementing or releasing a child, running the base and freeing. Exarust has the screen start but not the part refresh or the destructor.

## What Changes

- The part refresh stores the search result at the target only when the target exists.
- Destroying runs the base destructor and frees on positive flags.
- A child above one is decremented; a child at one or below is released.
- The base always runs.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the part refresh and destroy.

## Impact

- `exa_game::create`, from `refresh_creation_part` and `destroy_creation`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 30% to 32%.
- Evidence: the 64-byte refresh: the view target, the part search on the body's model and the store of its result. The 137-byte destructor: the flags guard, the destructor slot, the held child, its count and clear above one, the release below, the base and the free slot. No new corpus files.
- Out of scope: the search, release and free bodies.

## Phase

Phase 5, character creation. The part refresh and destroy.
