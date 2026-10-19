# Proposal

## Why

Phase 7 credits a match's template experience at tier 0 to the first combatant. The award also scales with the match tier and is split by what each fighter contributed, and the board has seven special templates besides the six standard matches.

## What Changes

- A shared finish multiplies the template by 1 + tier/4 and by the damage share, adds half when a trainer is hired, and splits that pool by contribution.
- An empty share list still credits the whole tier-0 pool to the first combatant.
- The special board lists Challenger, Elimination, Valiance, Valour, Captain, Brawl and The Beast with their recorded fee, prize, side and experience.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: tiered experience, the contribution split, and the seven special matches.

## Impact

- `exa_game::career`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the match-template table and the experience award (pool = (tier·0.25 + 1) · template · damage share, plus half with a trainer, split by contribution). No format field changes, so the corpus step is a skip.
- Out of scope: the hire price, how a tier scales a fee or a prize, wages, seasons, and tournament generation.
