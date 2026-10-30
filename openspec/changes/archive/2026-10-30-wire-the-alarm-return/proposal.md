# Proposal

## Why

Phase 6 (AI) of docs/PLAN.md. The `exanima06` alarm zones wake the statue
golems live and the golems now have bodies that fight, but the original's
way out of a lost fight had no live caller: in Exarust the woken golems
fought forever, nothing put them back to sleep, and the player could not get
past the gate. In the original, a story player knocked down with red health
left is revived five seconds later, and that revive first runs the alarm
return: the awake golems near the player go home and fall asleep, and the
player wakes outside the alarm box.

Evidence (REA, Ghidra listing of `Exanima.exe`): the story hook (the dead
condition `0xA00` with red health of at least 0.01 starts the knock-out,
less ends the game), the knock-out hook (five seconds, then the revive
behind the fade), the story revive (the world's revive pass, then the save,
the combat mode off, the body's revive, the held wake), the world's revive
pass (its first call is the alarm return, only while the alarm's armed byte
is set), the alarm return (a 3000 cm radius query around the player with
the homeward callback, then the `Alarm_` lookup, the player's pose reset at
the post with motion 0x1d, its condition `0xA00`, its tone and stance factor
0), the homeward callback (an awake guardian whose condition lacks `0x800`:
tone, stance factor and a third word 1, condition 0, Stand 2, pose reset at
home, sleep), the guardian sleep, the `Alarm_` lookup, the pose reset and
the skin-material glow write. Corpus: the shipped alarm items are named
`Alarm_` and their pass in hex.

## What Changes

- `exa_ai::golem`: the homeward callback skips a dying guardian (the
  condition word's bit `0x800`, formerly read as an alarm word), writes the
  tone, stance factor and condition; `alarm_return` composes the return
  (query, homeward callback, lookup, the player's reset).
- `exa_world::AlarmVolume::alarm_node`: the zone's node is an `Alarm_` node.
- `exa_physics::body::Ragdoll::carried`: whether the next aim carries the
  body with its root (a standing body moves with its root; others are
  placed).
- `exa_game::flow`: the story revive queues `Step::AlarmReturn` before its
  save; `FlowSystems` orders other plugins against the flow; the
  `StorySession` debug resource runs a play level as a story session.
- `exa_game::golem_alarm::return_from_alarm`: the live alarm return; a log
  line when a carried pass lets the player through a zone.
- `exarust`: `--story-session`, `--give id,...` and `--fell-player frame`.

## Out of scope

- The glow fade of the skin material `go` (logged, no live skin-material
  parameter in the renderer).
- The rest of the world's revive pass (every brain's reset: yellow healed
  to red, route reset; every body's wake).
- The pose reset's transition bytes, angular words, the parts' reached bit,
  the body links' re-measure and the frame at the body's standing height
  (Exarust keeps the character root's height).
- The wake's home angle read from the model's turn (it can differ from the
  body angle the original reads).
- The player's get-up after the revive's held wake.
