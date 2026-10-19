# Proposal

## Why

Phase 2 runs a combiner, a pair and a sequence. A broadcast does not yet walk the listeners and forward the signal.

## What Changes

- A broadcast signals every tagged object whose door link equals the relay, including the source.
- A door link that does not match is skipped even when the operative target matches.
- Without a door link, an operative is signalled when its target equals the relay, except the source itself.
- An untagged id is skipped. The value and the relay id are forwarded in list order.

## Capabilities

### New Capabilities

### Modified Capabilities

- `object-physics`: the relay broadcast.

## Impact

- `exa_physics::relay`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the broadcast step. No format field changes, so the corpus step is a skip.
- Out of scope: carrying out the close.
