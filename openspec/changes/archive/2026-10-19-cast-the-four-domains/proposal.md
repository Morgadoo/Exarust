# Proposal

## Why

Phase 7 lists four thaumaturgy domains and Exarust has no playable effect for any of them.

## What Changes

- Displacement places the body at a point (the long door) and spends 0.20 focus.
- Energy shocks: yellow health drops by the arc's strength, not again for 150 ms, and the cast spends 0.10 focus.
- Force blasts: the knock along the horizontal aim is 60, and the cast spends 0.12 focus.
- Mind calms: each concern drops by 30, aggression by 10 and fear by 50, floored at zero, and the cast spends 0.3 times 0.14 focus.
- A cast without enough focus changes nothing.

## Capabilities

### New Capabilities

- `thaumaturgy`: one effect for each domain.

### Modified Capabilities

## Impact

- `exa_game::thaum`.
- `docs/PLAN.md`.
- Evidence: the power tables (costs and factors) and the calm, blast and shock notes. No format field changes, so the corpus step is a skip.
- Out of scope: the charge-up, the other powers, focus regeneration, and the effect meshes.
