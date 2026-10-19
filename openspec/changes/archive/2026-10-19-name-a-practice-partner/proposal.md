# Proposal

## Why

Practice mode has no sparring partner. The next fight does not name one, and it does not restore health.

## What Changes

- A practice fight count advances by one and stops at 8.
- Novice counts 1 and 8 name opponent01 and opponent08. Expert counts 1 and 8 name opponent09 and opponent16.
- Count 8 is the last fight. A count already at 8 stays there.
- Red gains a tenth of the maximum, in double precision, and stays within the maximum. Yellow copies red.

## Capabilities

### New Capabilities

- `practice-ladder`: the next sparring partner and the health restored between fights.

### Modified Capabilities

## Impact

- `exa_game::practice::next_practice`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and `docs/formats/OTHER.md`.
- Evidence: the practice next-fight routine, the opponent name, the fight cap of 8, and the double tenth of maximum health. No new corpus files.
- Out of scope: limb impairment, a knock-out from low yellow, spawning the partner, and the expert-unlock setting.

## Phase

Phase 4, the first duel. The practice ladder's next partner.
