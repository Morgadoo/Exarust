## Context

How the original holds an item in a hand (docs:
[`docs/PLAYER.md`](../../../docs/PLAYER.md), Held items and their grips;
the race offset in [`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md),
Races).

**Hand class.** The routine that picks a held item's grip takes a weapon
class and a held slot: the second slot is always the off-hand grip (5);
a class whose high nibble is 9 (two weapons) the one-handed grip (1);
otherwise bits 4–6 of the class (0 fists, 1 one-handed, 2 two-handed, 3
polearm, 4 claws). At the equip it is called with the item's own weapon
class (the low 16 bits of its weapon flags); when the held items change,
with the character's class, only when that has hands bits 0x30: for every
shipped case both give the same grip.

**The table.** Six entries of 0xB8 bytes: flags (1 two-hand links, 2 a
stab link), an index, the main hand (0 right, 1 left), the offset (cm),
the ogre's offset (cm), Euler angles (degrees), the grip link count and
two links (point, slack), the two-hand links (item point to the other
hand, item point to the main arm), the stab link, and the grip-mode
callback (two-handed, polearm or none). Read from the installed
executable's bytes:

| grip | hand | offset | ogre | angles |
|---|---|---|---|---|
| 0 fists | right | 0 | 0 | 0 |
| 1 one-handed | right | (−12, −1.5, 6) | (−27.5, −7, 9) | (70, −15, 20) |
| 2 two-handed | right | (−13, −2, 6) | (−30, −2, 14) | (70, −15, 20) |
| 3 polearm | left | (11, −2, 2) | (11, −2, 2) | (110, 15, −170) |
| 4 claws | right | 0 | 0 | 0 |
| 5 off-hand | left | (11, −0.5, 6) | (30, −4, 14) | (110, 15, 160) |

**Placement.** With the record's race byte 3 (the ogre) the ogre's offset
is copied as it is; otherwise the offset × the character size (race size
× (1 − 0.05·height)). Then the race's three held-offset bytes × 0.1 cm:
x subtracted for the right hand, added for the left; y and z added (also
for the ogre). The angles go to the rotation builder (× π/180, in single
precision): rows X = (cos z·cos y, sin z, −cos z·sin y),
Y = (sin x·sin y − cos x·sin z·cos y, cos x·cos z,
sin x·cos y + cos x·sin z·sin y), Z = (cos x·sin y + sin x·sin z·cos y,
−sin x·cos z, cos x·cos y − sin x·sin z·sin y), the images of the item's
axes in the hand's frame (the x87 sine/cosine stores the cosine to the
second output, the sine to the first). A flipped held item has rows X and
Z negated. The world transform is the local one times the hand's.

## Goals / Non-Goals

**Goals:** the table, the hand class, the placement for every held item
Exarust holds (wielded weapons, the torch), unit and corpus tests pinned
to the table's numbers.

**Non-Goals:** the links as constraints and the grip modes (M24/M25), the
flip state, second weapons and shields, the half-sword hold.

## Decisions

- **The table lives in `exa_combat`** (`grip`), upstream of
  `exa_character` and `exa_game`, so the weapon spawn, the torch and M24's
  weapon bodies share it. The links are kept in the table now (data, no
  behaviour) so M24 does not decode it again.
- **Axes are copied as they are.** Exarust loads the skeleton and the
  models without any axis change, so the original's rows are the item's
  axes in the hand part's frame: glam's intrinsic `YZX` with (y, z, x).
  The torch's grip, the same convention, was measured equal to the running
  original's; the two-hander's upright blade matches the live capture.
- **The hand bone follows the grip** (`grip_bone`): a polearm's model is
  spawned under the left hand.
- **No flip yet.** `Grip::placement` takes the flip; every caller passes
  false until the record's held-item flag and the flip motion exist.
- **Classes past the table** (bits 4–6 at 6 or 7) have no grip and the
  weapon is not drawn; no shipped item has one (the original would read
  past its table).

## Risks / Trade-offs

- The one-handed comparison with reference shot 6 rests on its written
  description ("held low, angled down and forward across the body"):
  Exarust holds it low, forward and across the hips, about 25° up from the
  side. A retail still (RR13) would settle the tilt.
- Held items stay scene children of the hand: without the grip links the
  two-hander's and polearm's second hand follows its clip, not the haft.

## Open questions

- The entry's second word (4, 0, 1, 2, 5, 3) has no reader found
  (REA/`exe.asm` search of the stored grip pointer's reads); not used.
- The record's held-item flag word: bit 0x2000000 sets the held item's
  flip at load, 0x1000000 the half-sword hold (the writer RB11 asked for);
  read by M21/M25 follow-ups (exe.asm, three loader paths).
