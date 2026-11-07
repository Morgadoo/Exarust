## Why

PLAN Phase 7 (saves) and the importer's gap (RF_Import_RFC2). The round-140
followers lane carried a companion through a door but had no world
character database to register him in: his record kept his level id, and a
destination character with the same low id pushed him to a new number. The
importer's character reader also looked a world-referenced placement up in
the level's own records (by the low 28 bits), so it could find the wrong
character.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The importer's character reader (920 B) imports the level's embedded
  store (`0xCECDDB00`) into the local character database, then per
  placement looks the reference up through the databases' lookup: the world
  bit (`0x40000000`) routes the low 28 bits to the world store, any other
  reference to the local one; a reference neither finds is skipped. A
  record found in the world store is created only when its level word
  equals the current level.
- The save unit's character flush (633 B): a record with the local tag
  (`0x80000000`) stores the current level and its old id in its origin
  words, is deleted from the local store and added to the world store
  under id 0 (the first free id; the add stamps the store's tag on the
  record's id); every flushed record then takes the current level as its
  level. The transition runs it for the player and for every follower.
- The re-attach pass (352 B) stores the new current level on each
  follower's record.
- The character record writer stores the origin words as sub-chunk
  `0x00CDEC12` only when the level word is set; the reader reads both.

## What Changes

- exa_formats: the record's `reserved` pair is the traced `origin` (level,
  local id).
- exa_world: `placed_characters` routes world-tagged placements to the
  live world store's records (`LevelLoad::world_characters`) and gates them
  on their level; local references never match a world record. A corpus
  test finds no world-tagged placement in the shipped levels (477
  placements).
- exa_game: `world_characters::WorldCharacters`, the live world store (a new
  story holds the player's slot under id 1; a story load imports the save's
  `#worldchrdb`); the follower flush registers each follower
  (`register`), the host's re-attach stores the destination's id
  (`arrive`), and the story save writes the store's other records beside
  the player (`save_live::story_player_world_with`). A registered
  follower keeps its world id on arrival.
- exarust (viewer): the level load gets the store's records; a new story
  resets the store; the re-attach stores the destination's level.

## Impact

A companion who takes a door is written into the save's `#worldchrdb`
with a world id, his origin and his new level, as the original writes him.
Shipped levels place exactly as before (no world references). Not covered:
Exarust has no live local character database to delete him from, and its
level export does not write the scene's characters, so a load keeps him in
the store without placing him again.
