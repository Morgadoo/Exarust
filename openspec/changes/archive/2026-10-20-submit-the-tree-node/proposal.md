# Proposal

## Why

The power tree's submit appends one entry in state 0x21 when its node count is positive, then clears the state and index and runs the base. Exarust does not model it.

## What Changes

- Only state 0x21 and a positive count append an entry.
- The entry's first two floats are the value and the second over 5.0; its third word is the count.
- The state and index always clear and the base runs once.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the power tree's submit.

## Impact

- `exa_game::thaum`, from `submit_tree_node`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 277-byte submit's state guard, divide-by-five and clear. No new corpus files.
- Out of scope: the node count and base bodies.

## Phase

Phase 5, thaumaturgy. The tree's submit.
