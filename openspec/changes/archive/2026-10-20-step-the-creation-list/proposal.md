# Proposal

## Why

The creation screen steps its option list to the previous or next index. Exarust has the list rebuild but not the two steps.

## What Changes

- Stepping back rebuilds at the index less one when the index is positive.
- Stepping forward rebuilds at the index plus one.
- The step returns whether the rebuild ran.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the list step.

## Impact

- `exa_game::create`, from `step_creation_list`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 22% to 23%.
- Evidence: the 43-byte step back: the positive test on the list index and the rebuild at the index less one. The 34-byte step forward: the same call at the index plus one. No new corpus files.
- Out of scope: the rebuild body.

## Phase

Phase 5, character creation. The list step.
