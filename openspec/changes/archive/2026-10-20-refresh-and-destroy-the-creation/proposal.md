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
- Evidence: the 64-byte `0x1001313b0`: the target `0x180`, the part search on `0x160 + 0x2e0 + 0x18` and the store at `0x180 + 0x180`. The 137-byte `0x100131180`: the flags guard, slot `0x90`, the child `0x260`, the count and `0x18` clear above one, the release below, the base `0x10002aae0` and the free at slot `0x70`. No new corpus files.
- Out of scope: the search, release and free bodies.

## Phase

Phase 5, character creation. The part refresh and destroy.
