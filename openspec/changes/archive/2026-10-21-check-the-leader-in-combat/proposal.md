# Proposal

## Why

The combat state ends its tick when a following character's leader calls
it back; that follow check (its band, gait and route decisions) was an
input with nothing behind it.

## What Changes

- `exa_ai::combat::follow_check`: the band's ease toward 700 cm (toward
  500 cm plus 100 cm × how far ahead of a sprinting leader), 0.7 of it
  while following, the stop inside it, the gait outside it, the call and
  the replan.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-attack`: the follow check of a fighting follower.

## Impact

- `exa_ai::combat`.
- Evidence: REA on the original's follow check called by the combat
  state. No format field changes, so the corpus step is a skip.
- Out of scope: the route planning it asks for, the leader's choice (the
  player or the mind-controlling character), and its wiring (no fighting
  followers in the running game yet).

## Phase

Phase 6, the NPC combat AI.
