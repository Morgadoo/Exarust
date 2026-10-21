# Design

## Context

The damage routine branches on the hit's kind before its flesh path: a
clash (a weapon met a weapon) and a block (a shield or a torch). Exarust
has no weapon or shield bodies yet; a fighter's `Guard` stands for the
kind (`Parry` the clash on a parrying victim, `Shield` the block). See
[formats/OTHER.md](../../../docs/formats/OTHER.md) (Damage).

## Goals / Non-Goals

**Goals:** the block's numbers bit-exact; the guards' effects on both
fighters as the original's.

**Non-Goals:** setting a guard from a parry or a raised shield; stamina
regeneration.

## Decisions

- **The arm's stamina** lives in the guard (`ShieldGuard::stamina`) and
  the hit reaction writes it back after each block.
- **Holder inputs** (`Holding`): Synchrony, Persist and Counter from the
  holder's perks; whether it counters this attack is false (no counters);
  its clock is its attack clip's swing clock when it attacks, else 0.
- **No push**: the block path shoves nothing; the hit reaction sends a
  shove with no velocity, carrying the daze and the sag.
- **The low-health check** runs under 0.05 yellow whether or not the
  block took any; a weapon parry runs none.

## Open questions

- Which fighters hold a shield up or parry: the parry trigger and the
  shield's grip (parry notes); answered with the parry lane.
- The clash's sounds and the parry's end: REA on the parry-end routine.
