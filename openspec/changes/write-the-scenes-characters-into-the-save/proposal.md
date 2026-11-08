## Why

PLAN Phase 7 (saves), XA_SaveGames and SG_Character_DBs. The round-141
lane registered a carried follower in the world character database but left
two gaps: Exarust kept no live local character database (the level's own
store) for the door flush to delete him from, and the story save's level
section kept the loaded section's characters, so a saved follower (or any
NPC that moved or died) was not placed again by a load. The world store's
player slot also stayed the new story's reset record.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The save routine flushes every simulation character into its record
  before the sections (yellow and red as shares of the maximum, the focus,
  the 0xFFFF word, the heal-pool `0xCECC0001` and potion `0xCECC0002`
  entries), writes `#worldchrdb` after flushing the world store (every live
  character re-serialised), then `#playerdata`, `#worldobjdb` and the
  exporter's write of the whole scene under the locale name.
- The exporter's walk collects every saved node with the character flag
  into its character list; the character section (`0xCAC00001`) stores the
  health of each non-class-1 character into its record again, flushes the
  local character database and writes it as `0xCECDDB00` when it holds a
  record, then writes each character's placement (`0xCAC0CA00`): its
  record's id, the body's x, floor and z, the heading, the condition word,
  and the pose (`0xCAC0EB00`) for a dead body or one flagged so, the AI home
  (`0xCACE0F77`) and the motion state (`0xCAC0EA00`) in save mode.
- The local character database is emptied by a named level's load and
  filled by the importer's character reader; the door flush deletes a
  follower's record from it (round 141); the shared character's release
  forgets it in its own store by tag. The other writers found are the
  arena's (the opponent round clears it; the arena's death path adds a
  stripped copy of the dead player to it).

## What Changes

- exa_game `world_characters`: `LocalCharacters`, the live local store,
  rebuilt from the level's section when a load puts a new `LevelSource` in
  place (`track_local_characters`); `remove` (the door flush's delete),
  `store` (the save's flush), `section_body` (flushed and saved for the
  section). `WorldCharacters::refresh_player` puts the live player over the
  player's slot; `store` is public for the save's flush.
- exa_game `follower_travel`: the flush deletes a level character's record
  from the local store when it joins the world store.
- exa_game `level_save`: `LevelSaveScene::flush_characters` stores every
  scene character's live possessions and health into its store (world or
  local); `characters_after` rewrites the section's placements from the
  live scene (a moved or dead character takes its live place, heading and
  condition; one whose record left the store is left out; an arrival is
  added with its world id); the story save's section takes the live local
  store as its character database.
- exa_game `save_live`: both story save paths (new slot, flow save and
  checkpoint) run the flush first and pass the characters to the section;
  `scene_character_record` is the shared flush of one record.
- exa_formats `rfc_level::export_characters`: a world placement takes its
  node name from the world record with its full id.

## Impact

A follower carried through a door and saved is written into the save's
level section with his world id and placed again when the save is loaded;
NPCs that moved or died are saved where they stand. Not covered: the pose of
a character who died after the load (the stored pose is kept), the motion
and AI-home chunks of a moved character (the stored ones are kept), the
follow role after a load, and the arena's local-store writes.
