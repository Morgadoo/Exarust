# Proposal

## Why

A fighting place scores distance only. The original also scores the danger of an incoming blow, then scales that danger and the close term by skill and feral level.

## What Changes

- A live side swing scores danger from the milliseconds until the sweep reaches the candidate: 4 at the edge, less one three-hundredth per millisecond under 901 ms, then 1 until 10001 ms.
- A live thrust scores 1 inside its arc and 8 outside it, while closer than (skill + 3) × 20 plus the opponent's reach.
- Skill and feral scale the danger. Feral 1 halves the close term. A dull fighter scores no danger and skips that scale.
- Outside a fight the strike term stays 0 and the danger stays.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-attack`: the danger of an incoming blow once a fight has started.

## Impact

- `exa_ai::place::score_place`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the combat position score's danger term. No new corpus files.
- Out of scope: the approach term, the recovering close tweak, publishing an arc from a weapon hand, and a blow calling the knock-out.

## Phase

Phase 6, AI. The attack-arc danger.
