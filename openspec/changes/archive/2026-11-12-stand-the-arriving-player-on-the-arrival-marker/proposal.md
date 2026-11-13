## Why

Phase B lead B0.2 (docs/roadmap/phase-b-close.md): a save made after a level
exit recorded the new level (`exanima02`) with the old level's player
position (`exanima01`'s exit coordinates), so a continue could stand the
player anywhere.

## What the original does

REA and `exe.asm` of the story level exit, its checkpoint callback, the save
writer, the transition's load action, the level-change load, the player's
spawn placement, the level loader's transition-marker test and the save load:

- **The exit's save comes first.** The level exit gathers the player, then
  runs the checkpoint callback (below level 7 with checkpoints on, level 6
  always) or the plain save, and only then opens the transition whose action
  loads the destination. The save writer names the level section and the
  header by the current level's name, which only a finished load replaces:
  the exit's save records the level being left and the player standing in
  its exit zone.
- **The arrival.** The level-change load keeps the id of the level it left
  and the exit's entrance word. While the destination's objects load, a
  transition marker (volume flags low byte `0x40`) whose entrance and
  previous-level words equal both becomes the spawn node (a later match
  replaces an earlier one). The player is then placed on that node's frame,
  or on the identity frame at the origin when none matched, probed onto the
  floor from 100 cm over it (300 cm down).
- **The continue.** The save load reads the saved locale and its own level
  section and sets the previous-level id to all ones, so no marker matches:
  the player stands at the saved position in the level it left, and the
  story continue's nudge moves it out of the exit zone.

## What Exarust did

The exit's save already matched (it runs on the frame of the exit, before
the destination loads, and records the level root then present). The level
change did not: the player was put at the destination's own start (or
`--at`) in the frame only, while the simulation kept the surviving player at
the old level's coordinates. A save made in the new level then recorded the
new locale with the old position.

## What changes

- `exa_world::PlayerMarker` keeps the marker's rotation rows.
- `exa_game::level_change::LevelArrival` (the entrance and the left level's
  `locales.rdb` id) and `arrival_placement` (the last matching marker).
- The viewer records the level it last played, posts the arrival with each
  level change, and on a story level's arrival (no flow placement, landing
  plan or arena layout) stands the player at the marker and posts the
  placement to the simulation. `--at` now places only the first level.
- The simulation's side of a placement: an arrival is applied once the
  destination's collision is in (not after the whole load); the character's
  pending follow and ground from the old level are dropped; a standing body
  carried by its root is left to the root's carry (moving its parts as well
  had moved it twice, off the level, where it fell to its death); any other
  body is displaced whole (particle histories, contacts and foot anchors
  too), then turned about the root. A trace logs the root and the pelvis
  for a while after each placement.
