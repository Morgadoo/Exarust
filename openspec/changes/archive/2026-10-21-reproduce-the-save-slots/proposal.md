# Proposal

## Why

The save package container was modelled, but nothing placed sections the
way the original does, read a whole `#playerdata`, found, ordered, created,
loaded or deleted slots, or applied the checkpoint, permadeath, legacy and
transition rules. Exarust's session save was its own `EXRS` file, so
neither game could read the other's saves. The first implementation of
this change also got several rules wrong (the gap scan, the in-place
offset, the header size, the reference bit, the object database magic's
position, permadeath, the transition condition, the scan's deletions, the
clock and the death threshold); the review corrected them against the
binary.

## What Changes

- The package writer's placement: the gap scan over every pair of entries,
  the sentinel included, keeping the last strictly larger room in wrapping
  arithmetic; moving a section whose room is short; recomputing the
  written entry's offset from the entry before; keeping a read sentinel.
- Opening a package as the original does (create when missing, reset a
  short or bad-magic file's header in place).
- The 52-byte story and arena headers, the character placement (reference
  with its database bit, position, heading, condition, sub-chunks), the
  tagged chunks, load masks dispatched on the masked id, the journal, the
  world database sections, the story start check and the arena's version
  conversion (sections, chunks and the player's position).
- Slot files: names, the arithmetic number, the scan's keep / skip /
  delete verdicts per mode, the most-recently-used order in
  `Exanima.set`, new slot numbers, deletion, loads (first existing file,
  checkpoint copied back), checked and unchecked saves, checkpoints,
  permadeath (the save deleted, the slot dropped without a checkpoint),
  legacy removal and the transition save.
- The game clock (signed tick difference) and the death threshold (health
  widened to double against 0.01).
- Exarust's session save written as a package whose `#playerdata` has the
  mode's header, the journal and an Exarust chunk; saving over a retail
  save keeps its placement, sections and the chunks Exarust does not
  produce.

## Capabilities

### New Capabilities

### Modified Capabilities

- `save-games`: package placement and opening, player data, slots,
  checkpoints, permadeath, transitions, the clock.
- `session-save`: the session saved in the package shape.

## Impact

- `exa_formats::save_package` (`PackageImage`, `PlayerData`,
  `CharacterPlacement`, slot names, world database sections, start check),
  `exa_formats::save_chunk` (slot order and number rules, death and
  checkpoint rules, `GameClock`), `exa_game::session`, `exa_game::slots`
  (new).
- `docs/formats/SAVES.md` (new), `docs/formats/OTHER.md`,
  `docs/formats/README.md`, both crate READMEs. The `docs/PLAN.md` note is
  handed to the integrating session.
- Evidence: REA (Ghidra 12.1.4) on `Exanima.exe`: the package open, create,
  read, write, gap-scan, delete, copy and file-delete routines; the slot
  scanner, order, new-slot, delete, load, character-screen load, save,
  checkpoint, menu save, transition save, permadeath, legacy pass and
  `CheckpointSave`; both player-data readers and writers; the character
  placement reader and the record lookup; the clock. Arithmetic was read
  from the instructions. Two saves the retail game wrote on the
  developer's machine (story and arena, read in place, not copied) rebuild
  byte for byte and match the header and placement layout; the tests use
  synthetic packages of the same shape and a test reads `EXANIMA_SAVES`
  when it is set.
- Out of scope: exporting the scene, the world character and object
  databases' contents, the level sections and their import, the
  character-screen record load, the player name lookup, followers and
  migration across levels, spawn and portal placement, scene creation, the
  menu scene, the fighter kits, the arena table payloads and id passes, the
  scene environment presets, the GUI hooks and the script registration.

## Phase

Phase 7 (game modes and content), the save-games milestone: the save
system's slot and package rules.
