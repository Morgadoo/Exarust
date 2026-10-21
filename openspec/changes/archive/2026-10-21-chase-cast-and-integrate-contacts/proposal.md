# Proposal

## Why

The NPC combat state ran with three hooks unanswered: the chase route, the
power cast and the run stop. Its brain had the contact classifier and the
target pass's choice, but not the per-tick contact integration, the rest
of the target pass (the clean-up, the scripted focus, the encounter and
greeting, the round-robin refresh), the nearby-character list or the AI
tick's morale. A coding lane ported these; this change records them as
reviewed against the instructions.

## What Changes

- The chase route: the bark, the fresh and moved-foe tests, the goal on
  the foe's ground, the route through the navigation grid's planner (no
  second pathfinder), the reach test and the unreachable mark; the tick
  ends at the chase only while the route is followed.
- The plan turn: one brain plans a route a frame, the longest waiter
  next; the route start test (two points and more than 150 cm, or forced).
- The power-cast chooser at the first place and the cast's tear-down.
- The run stop before the drives and the can-act gate.
- The contact integration, the full target pass, the greeting's gates,
  the round-robin refresh and the nearby list; the AI tick's morale and
  the group alert's decay.
- In the running game: the duellist's chase (on levels with a grid, one
  plan turn a frame), the run stop, the can-act gate and a single release
  of a let-go swing. The brain's per-frame pass is not scheduled.

## Capabilities

### Modified Capabilities

- `npc-attack`: the chase route, the cast, the run stop and the can-act
  gate (new requirements).
- `npc-flee`: the contact integration, the target pass's clean-up and
  greeting, the nearby list and the morale (new requirements).
- `npc-steering`: the plan turn (new requirement).

## Phase

Phase 6, the NPC combat AI (docs/PLAN.md).

## Evidence

Reverse engineering of Exanima.exe 0.9.5 through REA (Ghidra provider):
the instructions of the chase route, the plan turn, the cast chooser, the
combat state's cast and run-stop sites, the contact integration, the
target pass, the nearby gather and insert, the greeting, the AI tick's
morale and alert, the run-stop helper, the vector helper it calls, the
can-act gate and the per-contact place scorer.

## Out of scope

- A power system for the cast chooser (Exarust has no thaumaturgy query
  in the running game); the chase's bark sound and the footprint lift
  around a plan.
- Feeding the brain's contact integration from Exarust's perception (the
  calming word, this tick's sighting and the concern target are not
  produced yet), so the brain's target pass is not scheduled.
- The scene-side half of the AI tick (activation holder, room occupant
  list, head look, gait, locomotion), the go-to-object states and the
  script hooks.
