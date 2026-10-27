# Proposal

## Why

The NPC brain's perception, contact integration, morale drift and target
pass were tested functions that the running game did not call: the mind's
own rating started a level character's duel, and the duellist perceived
its target with a stored attitude standing in for the role database.

## What Changes

- `brain::perceive_all` runs every frame for each brain: it clears its
  contacts' concern targets, closeness, view and noticed flag as the
  original's target pass does before perceiving, then runs
  `combatant::perceive` over every character within the largest perception
  reach (and every one it already knows) with that character's own
  contacts, the role database's attitude and the relations' trust, and
  writes the words the integration reads (this tick's sighting, the
  perceived attitude as the calming term, the threat estimate as the alarm,
  the concern target). A lost tracked target raises the brain's alert. A
  dead, asleep or removed character's concern and hostility are cleared on
  the brain; only the out bit stops the calming.
- `brain::ai_tick` drifts the morale, integrates the contacts and runs the
  target pass over every character seen this tick; the report stays on the
  brain and the combatant's contacts follow the brain's concern and
  hostility.
- The pass's entry against the duellist's target starts the duel and a
  calm pass leaves it; the mind's rating no longer does. The practice
  opponent, without a brain, still starts with the fight's setup.
- The contact integration raises hostility only from a hostile (negative)
  perceived attitude, as the instructions compare.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-perception`: the brain perceives, integrates and runs its target
  pass in the game, and the target pass decides the duel.
- `npc-roles`: a level character's duel comes from its target pass.

## Impact

- `exa_ai::brain` (`perceive_all`, `ai_tick`, `Npc::run_pass` takes the
  visible list and a distance function), `exa_ai::contacts`
  (`ContactRecord::of`, the integration's calming sign), `exa_ai` (`AiPlugin`
  order, the duellist's engagement), `exarust` (a `Combatant` on every
  living level character), `crates/exa_ai/README.md`.
- Evidence: the original's AI tick, contact integration (the sign-bit
  compare of the attitude word), target pass (clear, then per-candidate
  perception and classification) and perception (the dead and knocked-out
  branches, the lost-track alert), read from the instruction listing.
- Out of scope: one brain's pass a tick (the longest waiter), the
  line-of-sight filter, the nearby list, rooms and flight, retargeting the
  duellist to a third party, mind-power effects and masks.

## Phase

Phase 6, AI: the NPC brain's live triggers (docs/PLAN.md, next step 3).
