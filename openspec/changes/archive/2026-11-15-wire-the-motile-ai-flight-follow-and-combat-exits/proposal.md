## Why

Roadmap task B4 with B1 where it applies (lane `motileai`), the open
TMotileAI rows of the function ledger and findings F336, F338, F340, F341,
F343, F344, F345, F346 and F348 of the AI audits. A flight never ended for
lack of a threat and a cornered flight never returned to combat, the
target pass's leave did not restore the idle state, the combat state left
a lifted foe's footprint off the grid on its early exits and skipped its
down exit, the alert look-around never turned to a foe, the nearby list
gathered one 600 cm cell with no sight test, the AI tick's casting-flag
timeout ran only inside the duellist, and a follower inside its personal
stop planned nothing.

## Evidence

REA (Exanima.exe) and the disassembly, checked against the instructions
where constants or order matter:

- The flee state tick ends the flight when both the body's target and the
  object focus are empty; the escape end clears both. The flee route's goal
  callback, when the way out fails, makes the combat state current, sets
  the level to 3 and doubles the courage. The flee tick marks the body in
  combat every tick.
- The leave-combat routine restores the idle state and its step 1.
- The combat state's finally frame stamps the lifted foe's footprint on
  every exit. Its down exit (condition bit 0x200) takes the yaw of the
  body's facing as the look, heading and head look and copies the facing
  into the body's look directions.
- The alert look-around's foe is the body's target, its yaw from the
  body's place.
- The AI tick's 500 ms step clears the flag the cast chooser sets with
  the cast's start time (not a grab), before the condition gate.
- The nearby gather walks the level room's character list and the list of
  the room behind each exit, skipping an exit to the same room as the one
  before; the insert's sight test is the scene's light-reach callback.
- The follow state's closing-walk goal callback (constants 40000, 400,
  2500 and 300 read from the image): within 200 cm the goal is the
  leader's aim plus the leader-to-follower vector times 400 over its
  length, motion 0x14, mode 6, radius 300 past 50 cm; the plan is forced.

## What Changes

- The flee state reads the body's target, clears it at the end, marks the
  body in combat, and a cornered flight returns to combat.
- The target pass's leave restores the idle state.
- The combat state stamps the foe's footprint back on every exit and runs
  its down exit while the body is down.
- The alert look turns to the body's target.
- The nearby gather walks the level room and its exits' rooms through the
  sight test.
- The casting-flag timeout runs in the AI tick for every brain.
- A follower inside its personal stop backs off along the closing walk.

## Impact

- Crates: `exa_ai` (brain, flee, lib, combat, idle), `exa_game`
  (npc_search).
- Lockstep traces change in scenes with NPCs that flee, follow, are
  knocked down, or stand near other characters (their nearby lists).
