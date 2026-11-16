## Why

Lane `workergpu` (findings F547, F548). The first runs on the build
worker's GPU logged, on every level, that two types the
simulation's moved systems touch were not bridged: the vanish turn
message and the sound sets' variant cache. Bevy then skipped four
systems on every simulation tick: the casters' walk and the vanish turns
(no power progressed past its cast: no Vanish or Long Door vanished, the
Force Blast drew but never struck), and the plain bodies' impact class and
impact sounds. The headless tests run the systems without the bridge and
did not see it.

## What changes

- The vanish turn message is bridged (`bridge_message`), so the
  simulation world holds its queue.
- The sound sets' variant cache moves to the simulation world
  (`bridge_move_resource`); only the fixed-step body sound systems use it.

## Evidence

Worker GPU runs (`context/artifacts/2026-11-16-workergpu/powers-fix/`):
after the change the warnings are gone, a Vanish logs "vanished",
"enabled ... settling for 1000 ticks" and "shown" and the duellist
"leaves combat: it has no foe and no focus", and the Force Blast's loop
starts.
