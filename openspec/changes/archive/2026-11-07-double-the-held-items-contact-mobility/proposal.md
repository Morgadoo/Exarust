## Why

PLAN Phase 5 (the physical body). The TMotileDynamics gap lane rebuilt the
component's routine table against the current code and ported the largest
reachable remainder it could check against the assembly: the arm setter's
tail. The arm setter (routine #42 of `docs/re/motiledyn-map.md`) wrote the
arms' stiffness, relaxation cap and saturation in Exarust, but not its last
step, which sets the held items' contact mobility every tick it is called.

## Evidence

REA (Exanima.exe), read in the assembly and the pseudo-code of the arm
setter and its sixteen calls in the body tick:

- The setter takes a third factor, the held factor. After the two arm
  chains it walks the holder's held list only when the holder is armed
  (the weapon class word has a hands bit, 0x30) and puts each held body's
  contact mobility at its base times that factor; the shield's body takes
  the same whatever the holder holds.
- Every call passes 2.0 (the collapse, off balance, the daze, the power,
  the walk and run, the weapon switch and the relaxed blend, the last
  through the attack-arm easing routine's tail call) except the attack's
  branch (each attack slot) and the parry's, which pass 1.0. A shock and a
  kick make no call, so the items keep what the last one wrote.
- The base is written once when the item is equipped: a weapon's from its
  handling set-up (its mass-table contact mobility over the holder's
  scale), a shield's from its mass value or, when that is 0, its contact
  mobility.

## What Changes

- exa_physics: `Drive::held_contact` (the factor the muscle tune's arm
  state machine passed this tick: `HELD_CONTACT` 2.0, or
  `HELD_CONTACT_STRIKING` 1.0 in an attack or a parry, none for a shock or
  a kick).
- exa_character: `grip_runtime::scale_held_contacts` puts each held item's
  top part's contact mobility at its registered mobility times its
  holder's factor, the shield always, the other items only when the holder
  is armed; `held_contact_mobility` as a tested rule.
- docs/re/motiledyn-map.md: the routine table rebuilt (rows #21, #31, #42,
  #63, #70, #73, #81 and the rating's closed items).

## Impact

A held weapon or shield now yields twice as far in a contact with another
body unless its holder strikes or parries, as in the original. The arena
duel's lockstep trace changes (the weapons meet each other and the bodies);
exanima01's does not (no armed body touches another in that script). The
base stands in for the original's: Exarust registers a weapon body's
contact mobility from its rigid record, not from the weapon-mass table
(that table's use in the solver is a TMotilePart gap).
