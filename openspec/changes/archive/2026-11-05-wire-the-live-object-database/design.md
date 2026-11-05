# Design

Format page: [`docs/formats/OTHER.md`](../../../../docs/formats/OTHER.md),
Object database, The live tiers.

## One live database

The original keeps three tier objects: the base tier (no tag, the shared
library) and the local (`0x80000000`) and world (`0x40000000`) tiers, each
with its own library. Exarust keeps them in one Bevy resource,
`exa_assets::live_objects::LiveObjects`, so the level loader (exa_world),
the catalogue (exa_items) and the game systems (exa_game) share it.

- The base tier is read the first time it is asked for, as the original's
  lazy loader does.
- The level load queues the swap of the local tier: the level's `0xDBDB`
  records as a tier store (`Scene::local_database`) and its `0xDBCD`
  library. A copy made afterwards takes the first id the level leaves free.
- Decoded caches (`DollItems`, the item catalogue) refresh from the
  resource's write log: an in-place byte write names its record, anything
  else (a tier swap, a copy, a promotion) refreshes every tagged record.

## Saves

- Export: `#worldobjdb` is the world tier's writer and the **world tier's
  own** library's writer; the earlier reader put that chunk into the shared
  library, and the earlier export wrote the shared one.
- Load: the world tier and its library are replaced and migrated, or
  cleared without the section.
- Sync: the story's save, its checkpoint and every level change promote the
  player's carried items (worn list, out pair, stowed pair, the pack with
  hand repeats following) with the link remap table on, then rewrite the
  local doors' links. The arena save promotes the manager's record with
  the id remap table on and writes the career's loadouts back. The ids in
  the live possessions and the body's record are rewritten without a
  change notice: the items are the same and nothing is re-dressed.

## Deviations

- A level change after a world reset (an arena round, a scripted jump) is
  synced by promotion; the original copies the player's local items through
  a scratch database into the new local tier.
- The held bodies carry no item id in Exarust; the hands' promotion pairs
  are returned for them but nothing reads them.
- A tagged id the live database does not hold (an arena actor's own
  database) still reads its template's record in `DollItems`.
- The container popup's write sizes the section to its children and
  writes its size word [Exarust's: the popup's write path is not traced].

## Open questions

- The popup's container write path (not traced; answer with REA on the
  item container's close).
- The arena carry's hand-pair table: the decompiled pair loop stores the
  first pair's ids as the second pair's replacements (to be checked in the
  assembly before porting).
