## Why

Phase 4 (Combat and items) in docs/PLAN.md, TMotilePart, the motilepart
gap lane (round 139). The round-126 routine table left two partial pieces
this change closes: the attack starters' end of an ongoing parry (a
fighter that attacked from a parry kept its parrying flag, so the next
parry check could start the parry clip again over the attack), and the
living link iteration's support pins, which Exarust pulled once before
any link instead of after each part's own links.

## Evidence

REA (Ghidra and the objdump listing of `Exanima.exe` 0.9.5):

- The two side starters, after their motion start, test the parrying
  flag: when set it is cleared, and when the pick was not aimed at the
  foe's parry the parry's release runs (every held item's parry grip
  link written disabled, both hands and their parents losing the arm
  bit, the countered motion word −1). An aimed start keeps the grips and
  hands.
- The overhead and stab starters run the full parry end first when the
  flag is set: the release, the flag cleared, and (the condition word
  without 0x80) the stance chosen again (Defence when yellow < 0.01 or the
  energy is 0.05 under its most, else the combat idle) with a 40-frame
  start, which the attack's own start then replaces.
- The motion start, when the motion it leaves is an attack (5 to 12),
  clears the flag and runs the release: an attack's countered motion and
  a kept parry hold end when its clip is left for another motion, chains
  included.
- The living link iteration, per part: children first, the part's links
  (world links among them), then its support pin list (the thorax's two
  pins), then its extra pin record twice (the pelvis's, the same record
  pulled two times: checked in the assembly), then the foot contacts, the
  sole, the level contacts and the edges.

## What Changes

- `exa_combat::parry`: `AttackParryEnd`, `attack_parry_end`,
  `end_parry_for_attack` (queued by `start_attack` and the chained side
  start before the countered-motion write); `run_parries` releases an
  attack's countered motion and kept hold once the attack has been left.
- `exa_physics` body: `Ragdoll::support_pulls` once per iteration and
  `Ragdoll::pull_part_pins` after each part's links, in place of one pull
  of every pin before the links.

## Out of scope

- The slot-37 hit event for fists, add-to-world and the pin re-scale, the
  health write-back, the player's target component (the nearest fighter
  stands in), the heavy-claws hand mass write at the starters (the equip
  pass's write is the same value for an unarmed claw race).
- The weapon stat record functions and the character creator.
