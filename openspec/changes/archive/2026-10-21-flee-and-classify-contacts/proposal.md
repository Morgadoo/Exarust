# Proposal

## Why

The contact rating's choice tested both flight conditions before the
engagement and gave a negative tolerance the positive formula; the brain
kept no list of contacts, ran no target pass, and nothing could flee: the
flee start, the way out of a room, the flee state and the morale it moves
(courage halved, stress raised, courage doubled when cornered) were
missing.

## What Changes

- Port the contact classifier as `exa_ai::contacts::classify`, in the
  original's order: the stress flight, the engagement (threshold
  `7b² + (2 + b)`, `2 + b` for b ≤ 0; level 2 for an unreachable contact),
  then the concern flight or the watch, then the notice score. The contact
  rating's choice (`sense::decide`) runs it.
- The contact list's admission (twelve, the lightest replaced), the target
  pass's start and weighing, its outcome, the alarm mean and the sleeper's
  waking.
- `exa_ai::flee`: the flee start's morale, the way out of a room away from
  the threat, the fled-room memory, the flee state's tick, the cornered
  fall-back and the remembered exit after a plan.

## Capabilities

### New Capabilities

- `npc-flee`: the way out, the flee state and its morale.

### Modified Capabilities

- `npc-perception`: the classifier's order, the contact list and the
  target pass.

## Impact

- `exa_ai::contacts`, `exa_ai::flee` (new), `exa_ai::sense`.
- Evidence: REA on the original's contact classifier, target pass, flee
  start, flee route, room memory, flee tick and cornered goal
  (`context/re/npc-combat-state.md`). No format field changes, so the
  corpus step is a skip.
- Out of scope: the perception that fills a contact (another component),
  the path planner the flee state calls and the room graph it reads (no
  rooms in the running game, so no flight starts there), the greeting and
  encounter calls of the pass, the scripted focus, and the round-robin
  contact refresh.

## Phase

Phase 6, the NPC combat AI.
