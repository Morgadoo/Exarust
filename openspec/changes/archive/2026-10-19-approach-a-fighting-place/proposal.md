# Proposal

## Why

The fighting place scores distance and danger. The original also scores how much a candidate approaches the strike ideal, pulls that close term back while recovering, and publishes the arc from the weapon hand.

## What Changes

- The approach term is the eased fraction of the gap to the strike ideal, 0 past a band of 100 cm, or 150 cm when wide, plus (1 − skill) × 200 cm.
- A recovering swing subtracts 1 from the close term, not below 0, and multiplies the approach by (skill × 2 + 2) × 2.
- Action 5 to 7 publishes one side, 8 to 10 the other, and a hand at phase 0.4 or later sets the moving edge.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-attack`: the approach, the recovering close tweak, and the published arc.

## Impact

- `exa_ai::place::score_place` and `exa_ai::place::publish_arc`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the combat position score and the attack-arc publisher. No new corpus files.
- Out of scope: the facing scale, the idle-action clock, and a blow calling the knock-out.

## Phase

Phase 6, AI. The approach and the weapon-hand arc.
