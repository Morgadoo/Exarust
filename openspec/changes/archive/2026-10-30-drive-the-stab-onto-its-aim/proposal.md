# Proposal

## Why

This is Phase 6 (AI) of docs/PLAN.md: the golem guardians of `exanima06`, and
the duel's stab more widely. Woken golems now fight, but most of their halberd
stabs landed on the haft, 106–155 cm along a 176 cm weapon. The damage routine
gives a stab nothing unless the contact is within 14 cm of the tip, so those
blows showed impact 0.00.

The guess was that the duellist's reach or standoff for a long weapon on a
scaled body was wrong. The trace says otherwise. The equip refresh sets the
reach to the longest held record's W23 = 60·size + W22. The golem's size is
1.4 and the halberd's W22 is 143 cm, so the reach is 227 cm, the same as
Exarust's. The position score's strike ideal is that reach plus 10 cm for each
unit of the other fighter's size past 1, which is also the same. What Exarust
lacked is the stab's own steering. While a stab plays, the original's AI keeps
a stab aim on its target, and the body tick pulls the weapon onto the line
from the shoulder to that aim and drives it along the line. Without that, the
halberd followed only the clip, and its long haft swept across the target
before the point reached it.

Evidence (REA, Ghidra listing of `Exanima.exe`): the equip refresh and the
holding pass (reach = W23 = 60 × the character's size + W22), the combat
position score (strike ideal), the AI behaviour's stab-aim slot (a turn of at
most 0.0314 rad a tick toward the target from an anchor at (−20, 10, 0) cm in
the character frame, at most 0.6 rad up and 45° off the heading, the aim point
250 cm out), the executor's attack start (the aim yaw from the attack's aim,
pitch 0), and the body tick's stab arm drive with its caller (motion 0x0B,
race flag 8 clear, from clock 0.1, a primary weapon held, the cleave bit
clear). GPU runs: `exanima06`, and `exanima01 --opponent` with a sword and a
spear.

## What Changes

- `exa_combat::stab`: the stab aim (`StabAim`: its start, its tick) and the
  stab drive (`stab_drive`), with the original's numbers in tests.
- `exa_combat::swing::run_swings`: where the attack arms are set, a stab that
  has not landed and has an aim point runs the drive. The weapon point's push
  goes to the item's own body, and the left hand's push to that hand. A
  factory weapon has no body of its own (it is fixed to its hand's bone), so
  its push goes through the hand that grips it.
- `exa_ai` duellist: at a stab's start the aim starts on the attack's aim
  yaw. While the stab plays, the aim turns toward the foe's estimate each
  combat tick and is published to the body.
- `exa_anim`: the engine's quick arcsine (moved from `exa_ai::search`) and
  its pitch/yaw direction.
- `exa_physics::Ragdoll::part_centre`: a part's sphere-box centre in the
  world.
- Debug logs: each held weapon's record (length, sweet spot, reach, striking
  part) at its equip, each duellist's attack start (distance, reach, the
  chosen place's distance and approach), and the clock and motion of a blade
  contact. The hit log names the blow's kind and the weapon's length.

## Out of scope

- The player's stab aim (the player behaviour's own slot was not traced).
  Without an aim point the drive does not run for the player.
- The other aim nudges the swing routine applies to the weapon (edge
  alignment, the overhead's plane, vertical homing). They are reproduced
  in `strike_body` but not wired.
- Hits from the weapon body's own physics contacts, and bodies for
  factory weapons. Exarust still finds strikes along the held segment,
  which passes through a target standing inside the weapon's length. So a
  golem that attacks from closer than about 215 cm still hits with the
  haft.
- The resting statues' slump and the alarm return.
