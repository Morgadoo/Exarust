# Proposal

## Why

The fourth DeepSeek pass on the body dynamics class (lane `motiledyn4`,
docs/PLAN.md Phase 3, Bodies: skeleton, physics, control) added the
locomotion leg IK, rewrote the standing leg IK, and added the jolt, the
walk's footstep sounds, the travel phase, a sole-floor level query, a
stand-height block, the queued step's consumer, the release race's shock
caller and the muscles-off sampler's rules. Its review checked each rule
against the original's instructions. The standing IK rewrite was wrong:
it read the contact where the original reads the anchor and snapped the
corrections whenever a foot was anchored, where the feet routine never
snaps. The locomotion IK swapped the anchor and the contact and reversed
the hip's sign. The jolt keyed its threshold on moving instead of the
walking condition and missed the latch's countdown and the step end on a
latched jolt. The footstep used one draw where the original takes two.

## What Changes

- physical-body: Leg bend notes the double fold compare and the engine's
  arc cosine; new Locomotion leg IK, Jolt, Walking footstep sound and
  Travel phase requirements.
- Code: the corrections above; the sole floor stays the Ground Query's
  miss branch (no floor-flagged meshes in Exarust levels); the
  stand-height block and a duplicate queued-step entry are removed; the
  movement stage no longer queues the interim footwork velocity as a step
  vector; a release race's shock draws nothing and waits 500 ms.

## Evidence

The standing and locomotion leg IK routines, the feet routine's lift and
IK call site, the body tick's jolt block, the living sampler's footstep
and stand-height blocks, the post-step's travel phase, the force-release
helper and the muscles-off sampler's height ease and key-flag plant, read
as instructions in the motiledyn decompilation pack and the full
disassembly.

## Out of scope

The placement's snapping IK call, the per-leg lag the feet routine writes
while locomoting, the stand-height block (rest-pose quantities), the
Ground Query's floor-flagged meshes, the footstep voices as attached
sources and the splash, a caller for the queued step and the muscles-off
sampler, the pathfinder clamp of a queued step, and the force release's
feet re-anchor in the same tick.
