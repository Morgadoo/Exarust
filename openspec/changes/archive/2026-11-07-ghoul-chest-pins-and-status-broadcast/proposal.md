## Why

Phase 4 in docs/PLAN.md, TMotilePart, motilepart2 lane, third pass (round 98): the pin
and contact re-scaling and the status-effect list.

## Evidence

REA on `Exanima.exe` 0.9.5:

- The character size word is the race's size times `1 − 0.05·height` (the record's
  height, the creator's slider). Each motile part's size word is set to the scale and,
  for some bones, averaged with its node box's extent over a per-bone table. These are
  scalars that the body rules read. The body itself is not scaled by them: its rest
  particles come from the morphed model's nodes, and its links, pins and contacts are
  the template's raw records, copied by the motion binder.
- The re-scaling routine scales a part's thorax pins by the owner's character size and
  its foot contacts by the part's size. It recomputes their weights and world points.
- Two callers reach it:
  - The physique copy, which copies the template body onto the character's parts and
    re-scales each part. It runs only in the character creator's preview builder.
  - The character init for race classes 0x600 and 0x700 (Ghoul and Ghoul H). It halves
    the size word around a re-scale of the first part with pins, the thorax.
- So in game only the ghouls' chest pins change, to half the character size. Foot
  contacts are moved to the ankle by the muscle setup in any case.
- The status-effect list is the body's active power list. Its broadcast calls each
  entry's control with the event: own entries only for event 0 (the focus tick), every
  entry otherwise. Result bit 2 removes the entry, with the notice first, and the last
  entry takes its place. Bit 4 notices. Bit 0x10 keeps the object.
- Its callers:
  - event 2: the collapse (only when the body was not already dying), the kill-plane
    death and the fall death;
  - event 0: the focus tick;
  - event 8: the level load, for the player and for each follower.

## What Changes

- `CharacterStats::chest_pin_scale` (0.5 × size for the ghouls),
  `RaceMuscles::chest_pins` and `Ragdoll::scale_chest_pins`, applied by `with_race`.
  Exarust builds no physical body for the ghoul frame yet, so this is ready but unreached.
- `PowerCaster::{broadcast, was_dying}` and the `status_event` codes. `tick_casters` runs
  a pending broadcast over every entry before its own walk. Event 2 is set when the
  caster first becomes dying. `broadcast_level_change` posts event 8 to the player's
  caster on an in-process level change, through `FixedCommands`.
- The powers' own control routines were already ported. Strikes and the shell end only
  on 2; kinesis ends on 2 or 8 (`event & 0xa`).
- Not in: followers' event 8 (no follower spawn), the creator preview's re-scale (the
  doll has no physics body).
