# Proposal

## Why

The fifth DeepSeek pass on the body dynamics class (lane `motiledyn5`,
docs/PLAN.md Phase 3, Bodies: skeleton, physics, control) moved the
walking feet's lag into the feet routine, added the late foot's swing
flag, a Ground Query, the placement's leg snap, the body tick's timers,
and scheduled the muscles-off sampler for every limp body. Its review
checked each rule against the original's instructions and found: the
muscles-off sampler would have snapped every dying body onto its last
clip pose (the original's dead bodies sample nothing); the Ground Query
treated every level mesh as floor where the original meets only
floor-flagged nodes, raising walking grounds every tick; the backward lag
was negated twice; the swing threshold's cap used the maximum instead of
MINSS; the placement's reach used the wrong axes and its height the foot
instead of the root; and the timers read the scale and the tone where the
original reads the support and the balance, with the two adapters
swapped.

## What Changes

- physical-body: new Walking feet lag, Late foot swing flag, Ground
  Query, Placement leg snap and Body tick timers requirements.
- Code: the corrections above; the muscles-off sampler stays a tested,
  unscheduled function and a limp body loses its animator again;
  `Level::floor_piece` is false for the game's colliders.

## Evidence

The feet routine's lag, gait difference, Ground Query call and swing
block, the Ground Query itself, the placement leg snap, the body tick's
first lines (the adapters, the get-up hold, the look rate) and its
countdown and weapon-switch steps, and the body tick's sampler choice
and dead-condition exit, read as instructions in the motiledyn
decompilation pack and the full disassembly; the corpse notes for the
dead body's tick.

## Out of scope

Carrying the floor bit through the level's colliders, applying the
placement's frame height to the controller, the remaining body tick
blocks, a reader for the adapters and the look rate.
