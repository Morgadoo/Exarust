# Design

Rules: [`docs/formats/SAVES.md`](../../../docs/formats/SAVES.md) (Saving,
Character placement, Version conversion, Level transitions), read from the
save, player-data and arena routines' instructions.

## Decisions

- The world database export reuses the databases' own writers
  (`CharacterDb::flush`/`save`, `Store::write`, `Library::write`), whose
  layouts the instructions confirm; the gates are the record counts.
- The placement export takes the level exporter's `ExportCharacter`; the
  record pointer swap of the original becomes exporting a clone under the
  remembered reference. Flushing the record is reported, not performed,
  because the live database is the caller's.
- The list regeneration takes the employee's profession word beside its
  references (`StaffLists`), converts arsenal boxes to twelve-byte entries
  for the shared duplicate pass and back. The manager's list is labelled a
  guess (the field beside the stash).
- The live flow records what the running game knows (no package open, the
  archive answer, the spawn) and leaves the import to `exa_world`; it costs
  one small step list per level change, so load time stays flat.
