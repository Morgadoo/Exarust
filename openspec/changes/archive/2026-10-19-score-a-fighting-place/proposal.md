# Proposal

## Why

A fighting NPC closes to a fixed distance. The original scores the place: a close term and a strike term that is 1 on a plateau and 0 at 600 cm.

## What Changes

- The close term is `(2 − t) · t · 4` inside the other fighter's reach plus 150 cm, and 0 past it.
- Outside a fight that term doubles and the strike term is 0.
- In a fight the strike term is the squared fraction inside the ideal, 1 until one step past it, then a fade that is 0 at 600 cm.
- A large opponent adds 50 cm to the ideal. Wanting melee doubles the strike term.
- The duellist stands on that plateau and walks once past it.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-attack`: where a fighter stands once the fight has started.

## Impact

- `exa_ai::place::score_place`, called by the duellist.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the combat position score's distance terms. No new corpus files.
- Out of scope: the attack-arc danger, and a blow calling the knock-out.

## Phase

Phase 6, AI. The fighting place.
