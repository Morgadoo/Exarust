# Proposal

## Why

Spirit Form's body is a wraith (race byte 14), whose model has only the
twelve upper parts (pelvis to head, no legs) and whose race hovers. Exarust
built it as an animated model only: it stood where it spawned and the
player could not move it. Built physical, the body fell to the floor,
because Exarust's balance measure, lift and support pins all needed two
legs.

Phase: the Phase 3 physical-body milestone and the Phase 7 thaumaturgy
milestone (Spirit Form) of docs/PLAN.md (no PLAN.md edit in this change).

Evidence (REA, Ghidra on `Exanima.exe`, branches read in the assembly):
the balance routine's leg-count test (fewer than two legs: the feet
centre is the pelvis and the routine returns), the feet routine's hovering
branch (the probe, full support, the target pelvis height copied from the
sampler's standing height with no ground added, the lift servo run from
there), the living sampler's hovering standing height (the clip root's
height less 10 cm), the lift servo, the support pins' clamp around the
feet centre, the combat toggle's hover test, and the race table (only the
Wraith has the hover flag, `exa-tool races`).

## What Changes

- A living body whose race hovers runs the balance and feet routines with
  any number of legs. With fewer than two legs the balance routine only
  puts the feet centre at the pelvis: no lean, score, counter, recovery
  step or footwork, so the balance keeps its value.
- The hovering lift aims the pelvis at the clip root's height over the
  character frame less 10 cm, as it is (not over the probe's ground).
- A legless hovering body's support pins clamp around that feet centre.
- The wraith is built physical, with its race's hover flag, in the level
  and spirit spawns.
- A hovering player body's combat mode stays off.
- The mind log reports the lift's target and the character's ground.

## Out of scope

- The original's coordinates are sector-local; Exarust's one world frame
  stands in for every sector's (the hover target is absolute in it).
- The look-ahead's navigation grid slot for a hovering player.
- Legless bodies that do not hover (none ship).
