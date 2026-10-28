# Proposal

## Why

docs/PLAN.md Phase 4 (Combat and items): the object database is read at
every load. The previous pass left out the oldest layout's body converter,
the editor's look roll, the class-0 use action, the experience table and the
scene walks of the id lookup and rewrite. This pass adds them; review against
the assembly also corrected the converter dispatch, several body and
random-table offsets, the signed packed setter's mask and the condition
word's setter order.

## What Changes

- The `0xDBAC0D06` layout converts its bodies (old appearance parts into
  layers, categories and finishes into armour type and variant, a paint seed
  drawn per body) and its random tables' condition words, then the weapon and
  operative sections. `0xDBAC0D07` keeps converting the weapon and operative
  sections only (the loader runs its converter table from the matched entry
  down to the first).
- The random-table pass applies the condition setters one after another to
  the same word, as the original does.
- The signed packed setter keeps the count bits and clears bits 2–7 first.
- The class-0 use action (`ObjectDb::use_effect`), the experience table
  (`experience_for_level`), the editor's look roll (`exa_world::editor::roll`)
  and the scene lookup and rewrite of placed ids (`exa_world::object_entity`,
  `remap_placed_ids`).
- A placed light's condition share computed in double precision and cast
  once to single.

## Evidence

The loader's converter table (three magic/converter pairs) and its loop;
the body converter and its wrapper; the condition-word setters; the signed
setter's AND mask; the light pass; the use action; the experience table;
the editor's roll; the scene lookup. Hand-worked numbers are in the unit
tests (`crates/exa_formats/src/objdb/upgrade.rs`, `packed.rs`, `objdb.rs`,
`tiers.rs`; `crates/exa_world/src/editor.rs`, `lib.rs`).

## Out of scope

- The item factory's `@` weapon regeneration during migration.
- A real process generator for old-layout paint seeds (a fixed stream
  stands in; no shipped file is that old).
- Wiring the use action, the editor roll and the scene id rewrite into the
  running game (it has no editor, use screens or save promotion yet).
