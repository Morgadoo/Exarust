## Why

PLAN Phase 7 (saves). The save unit's follower rules (XA_SaveGames: the
migration list, the follower flush, the placement on the stored offsets and
the re-attach) were ported as plain functions but nothing called them: a
companion left behind whenever the player took a door.

## Evidence

REA (Exanima.exe), checked against the assembly: the game-main unit's
locale switch calls the save unit's transition for every door and portal.
The transition flushes the player, then walks the scene's simulation
characters: one whose AI activity word is 4 (the follow, the word the death
hook clears) and whose flags lack 0x800 joins the migration list once
("Duplicate simulation motile: <name>" under "Migration" for a repeat) and
is flushed at once (its record registered and its item ids prepared, the
player's flush). Each travelling character's node is set up (body
controller mode 8, detached from the old scene), the destination is
imported, the player is spawned at the spawn matrix (the portal node's,
else the identity), and each follower is spawned at the offset row-times
the matrix's rotation plus the player's placed position, with its own
floor probe (100 cm up, 300 cm down) and lift, flags cleared. The re-attach
pass then frees each one's UID manager, joins it to the new scene, stores
the new level's id on its record, writes 1 to its controller's phase word,
resets its controller, raises its minimum distance to 5 cm, and empties the
list. A save load, a new session and the arena's loads use other entry
points and carry nobody.

## What Changes

- exa_world: `level_lifecycle::BeforeDoorTeardown`, one-shot systems the
  teardown runs ahead of its despawns for a change to a level with no
  `WorldReset` (whichever system wrote the request).
- exa_game: `follower_travel`: the migration pass (`collect_followers`,
  registered as that hook) over the brains whose applied activity is the
  follow, through `save_level::migrations`; each follower's record with its
  live possessions and health, its local items promoted
  (`save_live::promote_followers`, now live), its held roles; the arrival
  frame, positions (`save_level::follower_positions` and `landing_feet`)
  and id; `restore_arrivals` loads the carried health.
- exarust (viewer): once the destination's collision is in, the travellers
  are placed around the player's landing and respawned with their roles.
- exa_game: the `--searcher` debug recruitment is spent once used.

## Impact

A companion who follows the player arrives with them in the next level and
keeps following. Exarust respawns the follower from its record (pose,
wounds and the walk start afresh); the original keeps the same object. The
world character database registration and the minimum-distance floor have
no live counterpart yet.
