## Why

`play arena1 --opponent --weapon ub05ssworda --fixed-frames` did not repeat:
two runs' opponent traces differed from tick 5, before and after round 85.
The grip lane also re-checked the grip placement's axis convention, which
the master notes marked inferred.

## Evidence

- `--sim-digest` with dumps: in one run the opponent's held polearm got its
  grip at step 6, in the other at step 9. The opponent was stood by
  `arena_live::stand_first_round` only after the load ended, then dressed on
  whichever frame the dressing budget and the paint jobs allowed. The
  dressing stage had counted only the player's outfit.
- Standing the round behind the loading screen made the runs differ from
  tick 1 instead. The spawn logs showed a differently built opponent each
  run (pelvis 93.9 cm vs 96.7 cm, limb volume 49496 vs 47216 cm³). The
  generator reseeds from `arena_match::counter_now`, the wall clock standing
  in for the original's performance counter.
- With both fixed: arena1 duel, 1271 common ticks, none differs. `play arena
  --opponent`: 1409, none differs. exanima01: 994 ticks, 41 bodies, none
  differs. A hub-launched match with the manager entered loads and plays.

Grip placement (REA): the placement routine sets the node's
translation from the table's offset times the character size (the alt
offset, unscaled, for character class 0x300), adds 0.1 times the race's
three held-offset bytes (x subtracted in the right hand), then calls the
rotation builder with the table's x, y and z angles. The
builder's sincos returns sine and then cosine. Its rows
are the item's axes. Exarust's `Grip::rotation` (glam YZX from (y, z, x))
gives the same three columns: a test now pins them for every grip, flipped
and not. The item's world is the local times the hand part's node matrix,
which Exarust's child-of-hand placement matches. No deviation was found.
A retail capture needs a retail save with a character holding a weapon.
None was made in the owner's prefix.

## What Changes

- exa_game: `arena_match::fix_counter`. A match the player enters stands
  its first round behind the loading screen.
- exarust: `--fixed-frames` pins the counter. The dressing stage waits for
  the round's bodies. Debug logs report the dressing stage and each
  dressed body.
- exa_combat: test of the placement turn against the original builder.
