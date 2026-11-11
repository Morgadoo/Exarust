## Why

Since the fixed step moved into the simulation's own world behind the copy bridge, lanes kept finding features
that silently did nothing or panicked there (round 160's revive reset, round 163's debug tools, round 168's Control
Mind handover). Round 169 swept the shipped game's features in the play build, free-running and in lockstep, with a
reusable grid script (`context/tools/feature_sweep.sh` and its scene list, outside git), and fixed what it found.

## What the sweep found

- **Every level with fires held its loading screen for 60 s.** The load holds game time (commit 7f1cd24ab,
  "Make story levels repeat tick for tick"), the flames step on game time, and the GI's build waits for every fire to
  have burnt two seconds: the wait could only end at the stage's 60 s timeout ("GI still not lit after 60s"). The
  arena took 62 to 64 s to load in every sweep scene, and the GI then built after the load.
- **The summoned demon's shell always took size 1.** The arrival (a fixed-step system, now in the simulation's
  world) read the demon's size from the `Physical` spawn request, which the body build removes; the bridge also
  reported `Physical` as touched by the moved systems and not bridged. The demon's size is 1.5.
- **Silent debug tools.** `--debug-slot` logged nothing in a level without devices, and the power tree's key opened
  the tree without a line, so a sweep could not tell a dead key from a working one.
- The grid host had no way to end an unattended batch whose scenes end by themselves.

## What Changes

- While the GI waits for the level's fires and the load holds game time, each unsettled flame runs the ticks it
  still needs at once (always the same count, so lockstep runs repeat); the GI then builds behind the loading screen.
  The wait for the fires is Exarust's own, so is the pre-roll.
- The summon's arrival reads the demon's size from its built body's muscles (`Drive::stats`), and logs it.
- `--debug-slot` logs how many devices took a crystal; the power tree's key logs that it opened the tree.
- `exarust grid --exit-when-done` ends the host once every scene's game has exited.

## Impact

- `exa_world::flame`, `exa_world::fire` (the settle count), `exa_game::thaum::summon_live`,
  `exa_game::teleporters`, `exa_game::power_screen`, `exarust::grid`.
- The body traces of lockstep runs are unchanged (the flames are not simulated bodies); the arena loads in about 3 s.
