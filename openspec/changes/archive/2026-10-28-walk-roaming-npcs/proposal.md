# Proposal

## Why

docs/PLAN.md, Phase 6 (AI): the roam and patrol routines, the alert
look-around and the wander and home-side picks were tested functions that
nothing in the running game called, so every level NPC with a roaming role
stood at its wait. The wander pick also blended toward a single room point
where the original blends between the door's two cells.

## What Changes

- The idle states' shared head runs in the AI tick for every waiting
  brain: the alert look-around (panic at least 0.2 with the body's
  in-combat mark, which fighting sets and the head clears), then the
  attention to the contact the target pass attends (notice score at least
  1, look window `clamp(1 − 0.1·score, 0.25, 0.7)·π`, 50° for a held
  focus), replacing the earlier look-at-the-target guess.
- A new walk system runs the roam and patrol phases on the
  one-planner-a-frame turn and the navigation grid, picks its goal only on
  its turn, walks the followed route through the NPC driver with the walk
  clip, and settles the look when a leg ends, before the pause draw.
- The wander goal is a quarter of the way from the door's near cell to the
  cell behind it; the room's exits for a roam list every door link with
  both cells and the room index behind it.
- A body the scene takes back mid-leg faces its route's goal.

Evidence: REA and the disassembly of the roam, patrol and wait states, the
attention routine, the wander and home-side picks, the vector blend
helper and the route start.

Out of scope: the idle-activity driver's use of the walk (no level object
uses it yet), the body's own locomotion turning the body, the physics
re-placement and the scene holder move, the standard sense's per-contact
threat and the combat state's remainder.
