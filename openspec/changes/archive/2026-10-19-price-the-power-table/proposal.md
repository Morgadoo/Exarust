# Proposal

## Why

Phase 7 casts one effect in each domain. It does not record the rest of the power table, or how a sustained tick and a repeated instant cast change the cost.

## What Changes

- The 33 recorded rows keep their ids and focus costs. The four shipped effects use the same costs as their rows.
- A sustained cost is the table cost times the tick in seconds, halved when the sustain technique is known.
- Repeating the same instant power with the repeat technique halves the table cost.
- Blast and bolt spend 1.2 times the cost, barrier twice, a mind power 0.3 times, vanish 80 times, and the long door once.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the power table and the cast-cost adjustments.

## Impact

- `exa_game::thaum`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the power-id table and the cast protocol's cost adjustments. No format field changes, so the corpus step is a skip.
- Out of scope: the tree layout, charge-up, the other effect meshes, and quick-bar dispatch.
