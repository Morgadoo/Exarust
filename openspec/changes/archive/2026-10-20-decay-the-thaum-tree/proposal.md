# Proposal

## Why

The power tree's decay subtracts two scaled terms, calls a virtual with a scaled runtime value, and routes mode-3 callbacks or a dim. Exarust does not model it.

## What Changes

- Mask 0x10 subtracts `(2 + field 0x168) × each float × 4` from 0x17c and 0x180, stamps the first start and returns 3.
- Mask 0x20 calls the 0x180 virtual with the runtime value × −10 rounded and returns 3.
- In mode 3 masks 0x200 and 0x100 call 0x250 and 0x248 when present.
- Otherwise a mask carrying 0x300 dims unless the dim bit is set.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the power tree's decay.

## Impact

- `exa_game::thaum`, from `decay_tree`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 316-byte decay method and its 2, −10 and 4 constants. No new corpus files.
- Out of scope: the virtual and callback bodies.

## Phase

Phase 5, thaumaturgy. The tree's decay.
