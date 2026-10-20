# Proposal

## Why

The level's mechanisms (the door, lever and push-surface bodies and the operative signal routines) had their single rules in place but not the whole routines: a door's and a lever's shape from their links, the stored word, the load setup of an operative, the node word a latch writes, and the precision the original uses for its thresholds. A first pass reproduced the routines; a review against the instruction listings found where it still differed.

## What Changes

- A door's shape: the hinge axis starts straight up and each qualifying pin replaces it, the last stop gives the radius and the half swing (with the engine's fast arcsine), any stop on the positive side makes the swing sign negative, and the frame's zero is the stop's anchor, so closed is the half swing.
- A door's step takes its creak, motor, arrival, shut, near, single-leaf and passage tests, and a slider's floor, step, ceiling, shut, pass and knock tests, in double precision. A single leaf whose single-precision angle is the float nearest 0.05 rad short now leaves. The knock's random share is on its pitch only, and the stored word is written only by commands.
- A lever's shape: the last pin farther than 1 cm gives the axle, and the detents are at −α and +α around the arm's anchor. Its step's thresholds are taken in double precision, the nearest detent rounds half to even, and the tick loop fades only while the lever returns. A report of 0 fades a linked light in and a report of 1 fades it out.
- A push surface's ramps, grind blend and thresholds are taken in double precision; its state is the object word's low byte as stored; each tick sets its links' smoothing to 0.1 and a grinding plate stays awake.
- An operative's load setup still runs its kind's part after the switch-off; a load fire takes its size and lift from its first target word; a signalled fire of value 0 has size 2; by day the light or its lit parent is put out; closing, locking and latching the doors under a node write bit 0 of its word.
- A placed object keeps its door and operative records in the running game.

## Capabilities

### New Capabilities

### Modified Capabilities

- `object-physics`: door and lever shape, the precision of the door, lever and plate steps, the plate state, the light direction, the operative load setup, the node word, and the records a placed object keeps.

## Impact

- `exa_physics::{door, lever, plate, mechanism}`, `exa_world::mechanisms` (new) and the placed-object spawn in `exa_world`.
- Crate READMEs of `exa_physics` and `exa_world`.
- Evidence: reverse engineering of the four components `TRBDDoor`, `TRBRotator`, `TRBPushSurface` and the `RF_Dynamics_Game` unit routines through the REA tools (pseudo-code and instruction listings, plus the engine's vector, angle and fast-arcsine helpers they call). No format field changes, so no corpus step.
- Out of scope: the world-link solver law that would let a mechanism body move, and so any system that ticks a mechanism or routes signals in the game; the fire preset spawn and its attachment; the electricity effect fallback; the navigation passage report; the per-particle drive of a door or lever; sound voice plumbing; clone, free and destructor routines.

## Phase

Phase 2, static world: the level's mechanisms (doors, M12; levers, plates and signals, M35).
