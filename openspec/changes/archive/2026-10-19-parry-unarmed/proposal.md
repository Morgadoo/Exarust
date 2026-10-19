# Proposal

## Why

An unarmed fighter does not yet answer a swing that is already coming, and a bite link is not yet pulled by the body solver.

## What Changes

- A valid threat is parried unarmed at speed 1 and blend 20. The clip follows the height table.
- Yellow under 0.01, a helpless fighter, a blow that is early, behind, cleaving, too big, or out of reach is refused.
- A bite above 0.1 opens a grab. The solver pulls the two parts together. A bite of 0.1 opens nothing.

## Capabilities

### New Capabilities

### Modified Capabilities

- `physical-body`: the unarmed parry and the held bite.

## Impact

- `exa_combat::parry` and `exa_combat::bite`, and `Ragdoll::pull_grab`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and `docs/formats/OTHER.md`.
- Evidence: the unarmed parry decision, the height table, and the breakable-grab pass. No new corpus files.
- Out of scope: armed parry grips, injuries beyond damage, and practice mode.

## Phase

Phase 4, the first duel. Unarmed parries, and a bite the solver holds.
