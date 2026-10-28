# Proposal

## Why

The sixth pass left the shield's block factors, the flipped record of a
weapon that starts turned over, the weapon switch's swap and the
equip/swap refresh out of Exarust. This is the seventh pass on the motile
part class (docs/PLAN.md, Phase 4, the TMotilePart entry), reviewed
against the executable's instructions.

## What Changes

- `exa_combat::damage::shield_guard`: a shield's equip derives the parry
  factor 0.5 + 1.5·(1 − w)² (w halved with Mobility), the leak 1 − block/2
  and the cost (× 0.6666667 with Endure) from its record, kept on the
  holder's `ShieldHold`. The fighter's standing guard is not changed.
- A weapon that starts flipped is placed half a turn about y and its
  record is built from the flipped side; the second weapon turns with the
  character's flip, which the main hand's attach sets.
- `condition::swap_due` and `grip_runtime::swap_on_switch`: on the tick a
  switch passes 60 % of its clip, two held weapons trade blade slots and
  the attacker's primary.
- `grip::refresh_hold` and `grip_runtime::refresh_holds`: the held weapon
  count, the first weapon's reach and aim height, the 0x521 flag and the
  parry gate, recomputed every tick into `HeldRefresh`.

## Evidence

The shield equip, weapon attach and held refresh routines read in the
disassembly (context/re/exe.asm) and the motilepart pack: the parry factor
is (1 − w)·(1 − w)·1.5 + 0.5 in single precision on both branches; the
attach sets the character's flip progress from the item only for hand 0;
the refresh counts the hand slots, which shields and torches do not use.

## Out of scope

Blocking with a shield (a blow on the shield's own body), the shield
parry, the shield's sector registration and sound set, the refresh's aim
point and per-item point copy, re-running the moveset on a swap, and any
reader of `HeldRefresh`.
