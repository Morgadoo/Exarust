# Proposal

## Why

Phase 7's story mode earns no experience and its arena shop has no shelf.
The original turns discoveries (new map cells, new item kinds, kills) into
skill experience, techniques and power levels for the player and the
companions, and into account-wide experience; it fills and rotates a
shop's stock from the object database by rarity, kind and rank and
settles each confirmed trade against the company's coins. Exarust has the
company, its copper and the shop line prices, and the arena's lesson, but
none of this. The story level hook also picks each level's track, volume,
ambience and indoor preset; Exarust plays the first level's track for
every story level.

## What Changes

- Discovery points: a new item kind by its rarity tier (10 to 2000), a
  killed character by its rank, a newly explored map cell 10, in the
  5 × 5 cells around the player on a 600-unit grid, once per level bitmap.
  Each point goes to the account pool and four times to the player and to
  every companion.
- Conversion once a second: a point per 50 ms of a character's cost
  (`4 + skill levels`) adds 2 to the trained skill, snapping to the next
  5000 with its technique; the player's paid cost also levels the power in
  training (one level per 4, capped by the word 0x1fff); a companion that
  levels gains 10 combat skill and 10 courage (to 100) and picks its next
  technique from the fixed table; the account pool converts at
  `(global / 5000 + 1) × 4` a point and stops at 25000.
- The account experience's worth and its inverse, its settings value, the
  companions' list, the fighting estimate, the discovery chunk of a save
  (written and read, recovering after an empty level name).
- The arena's lesson goes through the same skill lump.
- Shop stock: a first fill of 18 to 20, a rotation by stock size, the
  candidates and their weights (the original's quick logarithm and
  exponential), duplicates divided by 3, arms halved, the draw, new
  apparel and weapon copies, the duplicate check; the apparel copy's
  condition bytes; the merchant's category and rank options; the
  transaction (coins, tallies, sold and bought items).
- The object database's shop fields (listing word, parent, model rarity
  tier and rarity, size words, rank).
- The story level's track, volume, ambience and preset, and whether
  entering a level changes the music.

## Capabilities

### New Capabilities

- `story-experience`: discovery points, conversion, the account pool, the
  discovery save data.
- `shop-trade`: the shop's stock, the merchant's options, the transaction.

### Modified Capabilities

- `audio`: the story levels' music.

## Impact

- `exa_game::experience` (new), `exa_game::trade` (new),
  `exa_game::career` (the lesson calls the skill lump; `Company::restock`,
  `Company::settle`, the shelf), `exa_formats::trade_record` (new),
  `exa_audio::music` (`story_music`, `story_music_changes`, and
  `music_for_level` for story levels).
- `docs/formats/OTHER.md` (Trade fields), crate READMEs.
- Evidence: REA on the story experience unit and the trade unit (pseudo-code
  and assembly of every routine, the two option callbacks from their
  disassembly), the engine's quick logarithm and exponential, the object
  database's section reader and halving search, the coin packing, and the
  data tables (rarity points, the technique table, the option strings, the
  quick-math constants).
- Out of scope: the shop window and its layout, the description phrase an
  apparel copy is given, the weapon variant roll, the body refreshes a
  level triggers (technique bits, armour pass), wiring the story tick into
  a running level (no story world grid in Exarust yet), and reading the
  settings file.

## Phase

Phase 7, game modes: story experience and the arena shop.
