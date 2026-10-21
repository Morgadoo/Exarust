# Proposal

## Why

The damage routine's guarded paths were stand-ins: a parried blow and a
shield's blow went through the damage call with a made-up outcome (a
shield's yellow leak was cut a further tenth by the call, its stamina
never fell, a parry still ran the low-health check). The original's block
path takes a blow on the shield arm's stamina by its own rules and takes
its leak off yellow directly; its weapon parry deals nothing but locks
and sags the parrier. Phase 4 (combat and items), the damage routine
(lane following `finish-the-damage-routine`).

## What Changes

- `damage::block`: the shield's block path in the original's precision
  (the shield parry's 0.4·I, Synchrony's (0.3x + 0.3)·I while countering
  before 0.3 of the holder's clock, else 0.8·I; the leak (1 − S)·(0.4·I +
  0.4·x)·leak taken off yellow at 0.05, 0.025 under a potion; the daze
  (0.3 − 0.2·S)·Q and its stall factor by Persist and Synchrony; the
  shield parry's lock without Counter and its sag).
- `damage::parried`: a weapon parry's sag, min(I/2, 1).
- `Guard::Shield` holds a `ShieldGuard` (leak, cost, stamina, parrying);
  the hit reaction keeps the arm's stamina, applies the block (a daze, a
  sag, no shove's push), runs the low-health check under 0.05, locks the
  holder 600 ms; a weapon parry clears the parrier's attack state and
  locks it (not with Riposte). Both make a striker under 1.4 recoil.
- `damage::guard_outcome` is gone.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `combat`: Guarded blow; new requirements for the shield's stamina and
  the shield parry.

## Impact

- `exa_combat::damage` (`Guard`, `ShieldGuard`, `Holding`, `Blocked`,
  `block`, `parried`, `PARRY_LOCK_MS`), `exa_combat` (`react_to_hits`,
  `answer_low`).
- `docs/formats/OTHER.md` (Damage), `crates/exa_combat/README.md`.

## Evidence

REA (Ghidra) listing of the damage routine's block and clash branches,
constants read from the executable; a Python model of the listing gives
the tests' bits.

## Out of scope

Who holds a shield up or parries (no parry or shield guard is set in the
running game yet), the shield arm's stamina regeneration and its time
stamp, the counter-parry of this exact attack (no counters), the
weapon-on-weapon contact that is a clash, the block sounds, the parry's
end and the script hook.
