# Proposal

## Why

A collapsing fighter still keeps the weapons in its hands.

## What Changes

- The collapse start and the first dying tick drop every held weapon.
- Each weapon keeps its velocity, takes its own mobility, friction 0.75, and a drag of that mobility times 0.175. Its striker flags clear.
- The hands, the class word and the primary are empty. A second tick adds nothing. An empty pair stays empty.

## Capabilities

### New Capabilities

### Modified Capabilities

- `physical-body`: the weapons dropped when a fighter collapses.

## Impact

- `exa_combat::collapse`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and `docs/formats/OTHER.md`.
- Evidence: the collapse's release of both hands. No new corpus files.
- Out of scope: the free weapon body in the solver, and injuries beyond damage.

## Phase

Phase 4, the first duel. The weapons dropped at a collapse.
