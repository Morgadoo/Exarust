# Proposal

## Why

The arena load's id pass classified a re-added item onto its arsenal shelf
through a caller closure; the only implementation was an invented rule in
a real-data test (the first non-zero slot layer and whether the weapon and
shield sections parsed). The original decides from the object record's
sections: their presence, and the body's armour type byte.

## What Changes

- `exa_formats::objdb::Item`: `has_body`, `has_weapon`, `has_shield`, set
  when the record holds the section, whatever its contents.
- `exa_game::arsenal::ItemKind::of_item`: the record's classification
  (body → apparel with its armour type byte; else weapon section; else
  shield section; else no shelf).
- `exa_game::arena_ids::kinds_from_items`: the lookup over a saved world
  object database's records for `regenerate_lists`.
- The arena round-trip real-data check uses the classifier.
- Review: the lane's `save_scene::player_record_reset` and its
  transition step were removed. The original calls that reset from the
  save's load before the import, not from the transition flow, and
  `CharacterRecord::reset_loaded_identity` already reproduces it.
- Docs: `docs/formats/SAVES.md`, the `exa_formats` and `exa_game` READMEs.

## Capabilities

### New Capabilities

### Modified Capabilities

- `save-games`: arena item classification.

## Evidence

The load's classification routine in `Exanima.exe`: the record lookup by
id (a miss returns), the first body section's byte `0x1A8` compared with
3, 4, 5 and 6 (armour shelf 3, else clothing shelf 4), then the
section-presence tests for `0x4000` (shelf 0) and `0x8000` (shelf 2), the
scan of the shelf's five boxes for the id, and the add with the caller's
flag.

## Out of scope

Calling `regenerate_lists` from a live arena load (the running game does
not regenerate the company's lists on load yet); the player hook's effect.

## Impact

`exa_formats` (objdb) and `exa_game` (arsenal, arena_ids, arena_tables
test). Nothing in the running game changes.

## Phase

Phase 7 — Game modes and content: saving and loading a story or an arena
career (docs/PLAN.md).
