# Proposal

## Why

A first pass over the electricity unit's tendril emitter, the force
barricade, the force-shell aura, the golem guardians and the per-character
item effects added their remaining routines, but several rules came from the
decompiler's text rather than the instructions and some were invented: the
golem wake tested the wrong word, the radius query measured in three
dimensions, the alarm lookup measured a point distance instead of the
distance to the node's box, the item charge flashed on the wrong branch, the
hand effects were dispatched on a masked word with swapped names, the shell's
rigid copy and section filter were not the original's, the arcs kept a
clipped tendril's end point, and the barricade's navigation stamp was a
guessed full penalty that the running game never applied. This change
corrects them against the instructions of `Exanima.exe` and ports the
navigation grid's line stamp the barricade relies on.

This advances docs/PLAN.md Phase 7 (thaumaturgy in play: the force effect
nodes and the electricity unit's emitter), with the golem guardians for
Phase 6 (AI) and the item effects for Phase 4 (items).

## What Changes

- `exa_ai::golem`: wake, sleep, alarm trigger and its wake callback, the
  homeward callback, the `Alarm_` box lookup, the player's post and the pose
  reset, as the original writes them; the plane-distance radius query and
  the whole-index sector bound.
- `exa_game::item_effects`: the charge tick's HUD and counter on every tick
  under the source, the charge registration on the body's flag bit, the
  hand-effect dispatch on the power record's whole word (electric, glow,
  flame), odd-kind-only garment steps, the first-match material fade.
- `exa_game::thaum` (`ForceShell`, `rigid_shell`, `shell_fade_plane`,
  `shell_uniforms`) and `shell_draw`: the frame's state-4 order and one-shot
  colour, the rigid per-vertex copy, the inverse frame and the fade plane,
  the hum pitch and impact volume precision, the normaliser's sum order.
- `exa_game::thaum::arcs`: the level clip at the hit, the whole segment
  given to the character query, the node's update under a local-frame
  parent, and the world entry; the invented electricity uniforms and live
  arc light are removed.
- `exa_game::thaum::barricade` and `live`: the world entry's sector, hum
  pitch and hook rule, the teardown without invented focus handling, and the
  live barricade's stamp on the navigation grid.
- `exa_world::nav::stamp`: the line stamp and its removal.

## Capabilities

### New Capabilities

- `golem-guardians`: the golem guardians and their alarm zones.

### Modified Capabilities

- `thaumaturgy`, `item-effects`, `navigation-grid`.

## Evidence

REA (Ghidra) instructions of the 71 routines of the tendril, barricade,
force-shell, golem and item-effect groups, and of the helpers they call (the
squared plane distance, the point-to-box distance, the plane heading, the
engine's normaliser and matrix transforms, the sector lookup, and the
navigation grid's line stamp and removal with its offset table).

## Out of scope

- The electricity, force and shell GL programs; audio playback; GI and voxel
  light injection; spawning a force shell, a golem or an alarm zone in a
  level; dressing a character with the item effects.
- The character's own collision query for the arcs (the running game tests
  the parts' spheres), the AI hook bodies the golem routines install, the
  test of the player against an armed alarm zone, and the electric and flame
  hand-effect ticks (outside these groups).
- The scene's sector table (the barricade's entity records the sector index).
