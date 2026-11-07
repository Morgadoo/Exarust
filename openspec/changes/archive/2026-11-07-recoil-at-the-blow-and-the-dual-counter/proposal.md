## Why

PLAN Phase 6 (combat). The round-125 review of the combat-rules lane found
three defects: the held record's sweet spot ignored the flipped side, the
off-hand factory weapon was built without its item, and the attacker
recoil started its stance at the stall's end and never countered.

## Evidence

REA (Exanima.exe), read in the assembly:
- The record builder passes the side's W4 and W5 (bytes 0x35/0x36, six on
  when flipped) to the sweet-spot walk.
- The recoil: strength and stall 0.5, the released bit cleared; for class
  word 0x90 the continuation bit skips everything, and a clock above 0.4
  (compared in double) asks the counter routine, which, with none of
  0xA0012 in the flags, starts the other side's attack chained (motion 5–7
  to 8–10 and back) and skips the rest. Otherwise: flags 0x26, the end at
  round(drive·0.1·400) + now + 200, then the stance-by-energy routine with
  blend 40 (state 3 below yellow 0.01 or dazed, else 4), whose motion
  switch ends the attack and clears its clock, then both arms at 0.2.
- The follow-through has the same counter gate.

## What Changes

- exa_character: `held_record` takes the flipped side's striking part for
  the lengths; the off-hand factory weapon is built from its item's weapon
  section and takes the recipe's stats with the assembled extents.
- exa_combat: `SwingState::recoil_from` and `after_wound` take the motion
  and decide the counter; `react_to_hits` chains the counter attack, or
  starts the recoil's stance at once (blend 40, arms 0.2) and ends the
  attack; the executor stays busy until the recoil's end.

## Impact

The arena duel changes from the first recoil on: the recoiling fighter's
attack ends on the blow's tick instead of draining its clip until the end.
exanima01 is unchanged. Flipped plain weapons get the flipped sweet spot
(and reach); off-hand factory weapons look and fight as their items.
