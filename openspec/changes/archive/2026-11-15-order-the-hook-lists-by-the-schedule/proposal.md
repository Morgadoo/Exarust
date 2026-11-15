## Why

Roadmap tasks B2 and B4 (lane `hookorder`, docs/roadmap/phase-b-close.md):
the engine's hook-list and deferred-ring rows were left open by the
`engrt` lane because no one had shown that Exarust runs each list's work
at the original's point in its priority order. The player light read the
head where the last body tick left it, where the original's light hook,
called after the simulation's interpolation hook in the scene's frame
list, reads the head node where the frame draws it.

## Evidence

REA (Exanima.exe), against the assembly: every insert, remove, clear and
dispatch call of the two list kinds, grouped by the list they name, with
each registrant's priority and callback; the scene's frame dispatching
its frame list after the removed nodes are drained; the simulation's frame
hook moving every dynamic object between its last two steps under the
simulation lock; the light hook reading the head body's node; the
deferred ring's run loop going on only while the tail it found is above
the index as a plain integer. The map is `docs/re/hook-lists.md`.

## What Changes

- `exa_physics::schedule_order`: whether a built schedule forces one
  system before another, whether two unordered systems conflict in their
  data, and a schedule's place in the main frame.
- `exa_game::player_light` places the light over the head drawn at the
  frame's fraction (`Part::drawn`), as the mesh and the camera's focus are.
- Tests that check the scene frame list's and the game task list's orders
  against the schedule.

## Impact

- Rendering only: the light moves with the drawn head between ticks. The
  simulation and the lockstep traces are unchanged (no schedule order
  changed).
