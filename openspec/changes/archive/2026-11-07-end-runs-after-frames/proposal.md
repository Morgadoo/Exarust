## Why

Scripted runs pass `--frames n` without a screenshot (the grid's scenes,
the documented `--fell-player` and `--fell-nearest` runs), expecting the
run to end; it only ended after a screenshot, so grid children played on
until killed and runs of the same scene differed in length. The
exanima67 summoned demon also needed a trace of its held weapon to tell
why its swings meet nothing.

## What Changes

- exarust: `--frames n` without `--screenshot` ends the run after n frames
  counted as the screenshot's are (from the end of the load); a grid
  quadrant's game exits and the grid shows it ended.
- exa_combat: the debug aid `EXA_HELD_TRACE` (`held-trace`): a held weapon's
  part, blade tip, state, layer and group words, sleep, and its nearest foe
  part while its holder attacks.

## Evidence

Grid run (arena, `--frames 1800`): the quadrant logs `--frames: 1800 frames
played, exiting` and exits with code 0. The held-trace in exanima67: the
shaper's summoned demon's `SwordDmnA01` (record reach 220.2 cm, blade
203.4 cm, holder scale 1.5) is tied to its hand, awake, layer 0, never
phased; in its strike windows the blade never comes within 134 cm of the
player's parts and mostly points 140–180° away from the player.
