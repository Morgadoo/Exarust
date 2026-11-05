# Design

Format pages: [`docs/formats/SAVES.md`](../../../docs/formats/SAVES.md)
(Saving, The level section) and [`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md)
(Databases, The live tiers).

## The saved level section

The original's save exports its live scene with the exporter, whose
local-database chunk writes the live local tier. Exarust's scene model has
no converter back into the exporter's scene, so the save starts from the
section the level was read from and writes the live state over it:

- the local tier's writer and its library replace the section's own;
- each placed object the load spawned keeps its place with its live id,
  condition and placement. An object a body moves directly is where its
  node is; one over a body takes the body's motion since registration. A
  placement within a millimetre of the stored one keeps the stored rows
  [Exarust's, against the float noise of a sleeping body's frame];
- a spawned object that is gone leaves with what it held, and the later
  objects' container numbers close up; one never spawned stays; a drop is
  added at the end.

The load hands the save's section to the next load of that level
(`SavedLevelSection`), which reads it instead of the archive's file and
records it as the level's source.

## The importer's hook

Every level import migrates the local tier: the save unit's hook is the
tier migration for a save's section and the archive pass for the archive's
file, whose first step is that migration and whose second the container
fill. Exarust migrates in the tier swap and fills (writing the level's
records) only for an archive level.

## Arena promotion and the kit carry

The arena save's pass turns the id remap table on, promotes the manager's
record, the stash, the arsenal's 5 × 5 boxes and each employee's record
and list, rewrites the scene's entity ids and turns the table off. The
next practice fight prepares the player's kit into a temporary database
before the arena goes and equips it into the new local tier after the
import; while that database is held the level change does not promote.
The preparation's hand table stores the first pair's new ids for the
second pair (confirmed in the instructions), which `save_kit` already
reproduces.

## Deviations

- Props, doors, levers, characters and the settings chunk's live words are
  the loaded section's.
- The arena slot's load does not re-read the hub from the save.

## Open questions

- The exporter's walk over moved props and door leaves (answer with REA on
  the walk's node kinds when the scene converter exists).
