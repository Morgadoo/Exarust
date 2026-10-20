# Proposal

## Why

The power tree's key handler routes a non-shown state to the base, ignores keys while a hide deadline is still ahead, steps the selection on the next and previous keys and dims on any other. Exarust does not model it.

## What Changes

- A non-zero state goes to the base.
- A set hide flag with its deadline less 0x3b6 above the clock returns.
- Without key flag 0x2 the next global key steps +1 and the previous steps −1.
- Any other key dims unless the tree's dim flag is set.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the power tree's keys.

## Impact

- `exa_game::thaum`, from `key_tree`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 160-byte key method's base call, hide window and two key codes. No new corpus files.
- Out of scope: the step and dim bodies.

## Phase

Phase 5, thaumaturgy. The tree's keys.
