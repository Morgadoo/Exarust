## Why

Roadmap task B4, lane `savesflow` (the triage's L11: saves, flow and
arena). A save wrote the props as the level loaded them, so furniture the
player pushed went back to its place after a reload; a door's swung leaf
turned the whole door's saved placement; a save's level section kept the
per-triangle data the original's save exporter leaves out; the level
start's alarm reset cleared the pass word the original keeps; and a rise in
the armour skill never redid the armour, nor did a companion's rise refresh
its body.

## Evidence

REA (Exanima.exe 0.9.5), read in the assembly:

- The save unit creates its exporter once with save mode on and option
  word 4: the mesh writer tests that bit and leaves the triangle edge and
  flag sub-chunks out.
- The level start's reset writes the armed byte, the three point words, the
  alarm count and one view word; it never writes the pass word.
- The player's conversion, after a rise, refreshes the body and runs the
  armour pass on the player's body when the trained slot is the armour
  skill (1-based slot 3). The companion's conversion does the same on its
  own body, then writes its two attributes (each +10, capped at 100) times
  0.01 into its AI.
- The exporter's prop record writes the prop node's live matrix in its
  parent's frame.

## What changes

- `ExportOptions::save()` (triangle data left out) for every save export.
- `GolemAlarm::level_start_reset` keeps the pass; the viewer's level start
  calls `golem_alarm::reset_at_level_start`.
- `exa_combat::Fighter::wear` runs the armour pass; the dress keeps the
  worn protection on the body (`exa_character::WornArmour`). A rise in the
  armour skill (player or companion) puts `ArmourRefresh` on the body and
  `refresh_armour` re-reads Deflection and rebuilds the layers. A
  companion's rise puts its technique word on its body.
- Prop roots carry `exa_world::PropIndex`; the save writes each prop's live
  placement (`level_save::props_after`), and a body hung from the world by
  its links moves no saved placement.

## Not done

- The part records of an object's own bodies and cloth (a door's leaf, a
  lever's arm, live cloth particles) are still the loaded section's.
- The companion's two AI factors are not written (their AI fields are not
  named yet).
- The encumbrance is not summed again by the armour refresh.
