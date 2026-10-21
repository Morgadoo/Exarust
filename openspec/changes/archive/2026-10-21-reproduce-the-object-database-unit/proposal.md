# Proposal

## Why

The object database unit of the original keeps every item, door, lever and
container the game knows in three tiers (the read-only templates of
`objdb.rdb`, a level's own objects, and the objects promoted to the world
tier), with a string library each. Exarust read only the item fields of the
template records. A first port of the unit (records, tiers, strings,
instancing, promotion, migration, a writer) disagreed with the original in
its container and door-link offsets, its migration offsets and tests, its
string-library routines, its duplication and removal rules, its packed
setters' rounding, and its writer, which could not reproduce a single
shipped database; it also refused the older layout most arena actors'
databases ship in.

## What Changes

- Each tier is held as the original holds it: an id-sorted table over one
  payload blob, with the original's binary search, insertion, payload store,
  deletion, clear and writer, so every shipped database writes back byte for
  byte.
- The loader's count rule and its upgrade chain for the `0xDBAC0D08` and
  `0xDBAC0D07` layouts (operative and weapon converters, through the
  original's section rewrite and its section editing routines).
- The string library's loader, writer, getter, allocator, setter,
  add-reference and release as the original's.
- The id resolver, flagged and plain section lookups, the string resolver,
  the template clone, instancing, the instance template, the item
  dispatcher, local duplication, promotion with the id and link remap
  tables, the link and entity-id rewrites, both removal routines, the
  template and bulk helpers and the item-use test.
- Template migration by its assembly, with the appearance refresh over the
  procedural material sets and the condition refresh of protection, armour
  variant and value.
- The packed item fields and amount helpers, bit-exact.

## Capabilities

### New Capabilities

- `object-database`: reading, changing and writing the object database's
  tiers and string libraries as the original does.

### Modified Capabilities

## Impact

- `exa_formats::objdb` (now `store`, `record`, `tiers`, `migrate`,
  `upgrade` and `packed` submodules) and `exa_formats::objstrings`;
  `exa_items::instance::variant_scales` re-exports the shared table.
  `objdb::items`, `Item` and the section parsers other crates use are
  unchanged.
- `docs/formats/OTHER.md` (Object database), `crates/exa_formats/README.md`.
- Evidence: REA (Ghidra 12.1.4) pseudo-code of every routine of the unit,
  their assembly for the float arithmetic (packed setters, amount helpers,
  migration, condition refresh), data tables read from the executable (the
  upgrade table, the old operative class and kind tables, the material
  sets, the layer byte table, the condition constants); corpus statistics
  of the installed `objdb.rdb`, `objstrings.rdb`, the 22 level databases
  and libraries and the 20 actor databases.
- Out of scope: the body converter of the `0xDBAC0D06` layout (not shipped),
  the item factory's regeneration of `@` weapons during migration, the
  editor's palette randomiser, the experience step table, the scene walks
  of the entity lookup and entity-id rewrite (the id rule is ported), the
  class-0 use actions, and wiring the tiers into the level loader and saves.

## Phase

Phase 1, complete asset decoding: the databases milestone (`objdb`,
`objstrings`), which the save games of Phase 7 build on.
