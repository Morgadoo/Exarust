## Why

PLAN Phase 6 (combat). The combat-rules gap lane re-audited the 24 combat-rule
routines against the assembly. The blade-length routine that gives the weapon
record its length (W21) did not match: Exarust stopped its walk down the
weapon model's first-child chain at the first node that is not a rigid body.

## Evidence

REA (Exanima.exe), read in the assembly: the length starts at the weapon
node's box top (centre.y + half extent.y, single precision) and, while the
current node has children, steps to its first child and adds that child's
box top only when the child has a body whose class carries the rigid-body
bit. A child without one is passed over and the walk goes on to its own
first child. The sweet-spot routine is different: its loop condition
requires the next child to be rigid, so it does stop there (unchanged).

## What Changes

- exa_formats: `Lengths::armed` adds every rigid node down the whole chain;
  the test of a rigid node past a plain one now expects 70 cm; the shipped
  corpus test counts the weapons with a rigid node past a plain one (none).
- docs/formats/OTHER.md: W21 row.

## Impact

No shipped weapon has a rigid node past a plain one in its chain (the
corpus test asserts 0 over the 103 plain weapon models), so every shipped
record, and the arena duel and exanima01 lockstep traces, are unchanged.
