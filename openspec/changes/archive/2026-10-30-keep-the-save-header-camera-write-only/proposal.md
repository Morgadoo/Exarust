# Proposal

## Why

Ninth pass over the save-game unit (docs/PLAN.md Phase 7, saves). The
writers stamp the scene view's camera target and zoom into the 52-byte
header, a level change whose archive copy fails to import disables saving
until a level loads by name, the checkpoint routine saves before it copies,
an old arena save's player record is repaired during the conversion, and a
save without Exarust's own payload still carries the reader's globals.
Exarust wrote the header's defaults, ignored the disable flag in its live
saves, only copied on a checkpoint step, left the repair unapplied and fell
back to a bare session for a retail save.

Evidence: the original's assembly through the objdump listing: the story
and arena writers copy twelve bytes of the view's target and four bytes of
its zoom after the locale; both readers read the same sixteen bytes into
stack scratch that nothing reads again (the view is not touched); the save
writer's first instruction tests the disable byte and returns, ahead of the
manager, the scene and the double 0.01 health compare (NaN saves); the
flag is set only by the transition routine and cleared by the named-level
load and three other entries; the checkpoint routine probes for the
checkpoint, and only when absent refreshes the GUI, calls the writer
without the death check and copies the save whatever the writer did; the
arena reader repairs the imported record only when the data version is
below 9, worn list, each loadout's worn list, carried items, re-derive.

## What Changes

- `exa_game::session`: the header's camera pair on `Session`, written by
  both writers and dropped by both readers; `session_from_player_data`
  restores a retail save's clock, seed, start background, journal (or its
  default page), discovery, experience, memory, transition state and
  forced locale, or the arena's money, day and tables.
- `exa_game::save_live`: `SaveCamera` stamps saves (the running game does
  not set it, so defaults are written); the new-slot save and the flow's
  save step refuse while the last level change disabled saving; the
  checkpoint step saves (unless disabled) then copies, leaving an existing
  checkpoint; the arena load repairs a converted record before the
  identity reset; the story load restores a retail save through
  `session_from_player_data`.

## Out of scope

- Feeding `SaveCamera` from the chase camera: the original's view fields
  have no traced mapping onto Exarust's camera (its zoom runs 800 to 2000,
  the header's default is 1).
- The level section, the world object database export, the character
  screen's level import, the mode hook after the identity reset and the
  script registration of the checkpoint function.
