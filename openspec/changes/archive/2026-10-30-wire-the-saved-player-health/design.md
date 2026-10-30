# Design

## Where the body's health lives

Exarust keeps the original's body health on several components: yellow,
red, maximum and heal pool on `exa_combat::Fighter::health`, the potion's
expiry on `Fighter::potion_until` with the potion bit in
`exa_combat::StateFlags`, the focus on the player's
`thaum::cast::PowerCaster` (`body.focus`) and the condition word in
`exa_combat::Condition`. `save_live::body_health` gathers them into the
record crate's `Health`, so the store and the load stay the record
crate's ported routines.

## Only the player

The original stores every scene character's health before a story save.
Exarust's story save writes only the player's record (no live character
database exists), so only the player's store runs; the others are listed
as not done.

## Spawn order

`CharacterStats::derive` already turns the pools into yellow and red
(with the placement reader's full-pools rule for a living record that
saved no red). The player spawn then runs the record's load, which sets
red and yellow again from the record without that rule, as the
original's player spawn does; a fresh creator record (full pools, focus
0) therefore spawns with full health and an empty focus, as the
original's record constructor leaves it. Placed characters are not given
the load (their shipped records hold full pools and focus, and no saved
entries were looked for).
