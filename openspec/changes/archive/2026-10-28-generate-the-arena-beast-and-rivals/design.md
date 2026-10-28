# Design

Extends the arena career spec and the character record format
(docs/formats/RFC.md, character records).

## Beast record

`beast_record(tier, lcg, professions)` follows the spawn routine's order: the
first unit draw is smoothed into the pitch through `beast_plan`, which keeps
the tier items; the profession grid grant then steps the same sequence and
drifts the shared catalog, so later random NPCs see the drift as they would
in the original. The record's race byte comes from the profession word; the
body is built in the Ogre's own body model and moveset by the existing race
path.

## Reseed

`SpawnContext::clock_seed` is a function pointer: `clock_seed` in the game,
a fixed word in the tests, so records stay pinned. After a generation the
arena roll takes the sequence's state, as the original's global continues.

## Rivals

The field lives on the career (it is the tournament chunk's field); the
launch copies it, `ArenaScene::with_rivals` keeps it for the bracket, and
`rival_outfits` turns it into the team-indexed table `raise_fresh_match`
reads.

## Open questions

- Teams past the sixteen field slots read past the field in the original;
  no bracket reaches them, so they wear no company [guess].
- The hire profile's level byte uses the slot's tier (the event tier except
  for pugilism layouts, which no tournament uses).
- Item instance quality: answered later through `exa_items::randomise`'s
  grade once records can carry instances.
