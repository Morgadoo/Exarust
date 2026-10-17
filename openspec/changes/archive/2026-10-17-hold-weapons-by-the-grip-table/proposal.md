## Why

A held weapon in Exarust sits under the right hand by an invented grip: a
fixed 8.5 cm offset and a 90° turn about the hand's X axis, for every
weapon, every race and every size. Polearms are held in the wrong hand.
The torch alone uses two entries of the original's grip table, copied by
hand, without the character's size or the race's offset.

The original places every held item by one table of six grips, picked by
the item's hand class: the hand it goes in (the left for a polearm), its
offset scaled by the character's size (the ogre with its own), the race's
held offset, and Euler angles. The live retail capture of the player's
two-hander shows its blade upright before the chest, which the invented
grip tilts forward.

Phase 4 (Combat and items), milestone M4 "first duel"; the master plan's
lane C change L12 (milestone MS3, weapons as bodies), before M24 (held
weapons as bodies), which reuses the table's links.

## What Changes

- The grip table (`exa_combat::grip`): the six grips (fists, one-handed
  and the first of two weapons, two-handed, polearm, claws, the off-hand
  item and the second of two weapons) with their hand, offset, the ogre's
  offset, Euler angles, links to the hand, second-hand and arm links, the
  stab's link and the motion rules they follow; the hand class from the
  held slot and the weapon class.
- The placement: offset × character size (an ogre's own offset, not
  scaled), plus the race's held offset (x away from the body's middle),
  turned about the hand's X, then Z, then Y; a flipped item half a turn
  about its blade.
- A wielded weapon sits under the hand its grip names, placed for its
  holder's size and race (`RaceWeaponry` gains the race's held offset),
  replacing `grip_transform`.
- The carried torch takes its grips from the same table (the one-handed
  grip in the right hand, the off-hand grip in the left), now also for
  its holder's size and race; a human of average height holds it exactly
  as before.

Evidence: REA/`exe.asm` of the original's hand-class routine, grip table
(read from the installed executable's bytes), placement routine,
rotation builder and vector helpers; the installed `objdb.rdb` and
`races.rdb` (every weapon has a grip; the ogre's race byte; the demon's
offset); the live retail capture `r1-player-2h-stance-bloody-blade` for
the two-hander.

Out of scope: the grip links and the grip modes that switch them by the
motion (M24, M25: held items stay scene children of the hand), the flip
state (no weapon starts flipped; the record's held-item flag is not read),
second weapons and shields (no Exarust item holds them), the half-sword
hold.

## Capabilities

### New Capabilities

### Modified Capabilities

- `combat`: Wielded weapons (the grip's hand and placement); new Held
  item grips.

## Impact

`exa_combat` (new `grip` module), `exa_character` (`RaceWeaponry`
held offset and `grip_holder`, `grip_bone`, the weapon spawn), `exa_game`
(`torch` uses the table; its own `Grip` and constants removed).
`docs/PLAYER.md` (Held items and their grips; the torch), `docs/PLAN.md`,
crate READMEs.
