## Why

Roadmap task B4 with B1 (lane `motilepart`, docs/roadmap/phase-b-close.md):
the open TMotilePart rows of the function ledger. The live step reach was
rounded in a different order from the original's reach routine; the part
sounds were always picked as for a human (no golem or skeleton sets, no
golem layer class); the holding pass never saw the gauntlets' weightless
share; the golem's alarm-return reset and the AI's revive reset moved a
body without putting its held items back; and the drops wrote every part of
every kind of held item where the original writes the item's own body, by
kind.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The reach routine multiplies the single precision square-root-of-scale
  word (the init's SQRTSS of the scale) by the double product
  (feral factor) × ((1 − 0.2 × encumbrance) × 80) and narrows once; the
  feral factor is skipped only when the feral level compares below 0 (an
  unordered compare lets NaN through).
- The part-sound routine tests the profession word: 0x1000 (the golem bit
  of the race byte) writes the golem class 6 into the part's first covering
  layer and picks HitGlm; 0xff00 = 0x100 (the skeleton) picks SkelHit for a
  bare part. The foley branch runs on the golem-body byte, which the init
  sets for a golem race and the worn-effect registration for a golem suit;
  0x1200 picks Golem_Foley at 1 and 1, another golem race at 6 and 1.5, a
  suit wearer FoleyPlH. The armour pass calls the routine at its end,
  unconditionally.
- The armour pass zeroes the weightless share; the kind-6 registration
  (gauntlets) writes 1 while its crystal is in and unspent, 0 otherwise.
  The holding pass reads it.
- The golem pose reset and the AI revive reset's roaming branch each call
  the living placement and then the held-item re-place.
- The weapon's drop clears 0x30001 recursively; the torch's drop clears it
  on the item's own body; the shield's drop clears none. All three write
  friction 0.75 and the contact mobility from its base on the item's own
  body. The attach writes friction 0.5 on the item's own body.

## What Changes

- `weapon_record::step_reach` in the assembly's order (`attack_reach`
  reuses it).
- `exa_combat::sounds::PartSounds` from the race byte (now kept on the
  stats) and the golem suit, used by the limb impact and the footstep
  rustle; `mark_golem_layers` at the end of the live armour pass.
- `CharacterStats::weightless` from `WornSpecials::weightless`, set at the
  spawns and on every equip refresh, passed by `holder()`.
- `ReplaceHeld` requested after the golem's alarm-return reset and the AI
  revive reset's placement.
- `release_freed_grips` and `mark_held` write the item's own body; the drop
  restores the base contact mobility (`held_contact_base`).

## Impact

Simulation: the step reach changes by one bit for many characters; held
items of more than one part keep their children's friction; re-placed held
items after a golem or revive reset. Determinism and grid scenes owed (GPU
on hold).
