# Design

## Where things run

The AI tick (`exa_ai::brain::ai_tick`, once a body tick in `Update`) now
does, in the original's order:

1. The sleep entry, when the last role application chose the sleep
   activity: condition, flags and the drive's tone through
   `idle::sleep`, and a `StatueByte` message for the statue byte and gait
   word (the game keeps both as `golem_body::StatueRest`, which the
   posture stage already turns into the dormant drive).
2. The condition gate (0x800), the target pass with `asleep` = flags
   exactly 0x10.
3. The wake when the pass reports it: role 44 removed (the roles are
   applied again by `apply_new_roles` on the next frame, which stands for
   the original's re-application flag), `exa_combat::condition::knock_out`
   with no delay, `Ragdoll::knock_out`, a `KnockOut` timer for the body
   tick's `track_wake`, and the statue byte cleared.
4. The current state: the sleep tick while the sleep is the current state
   (not fighting); otherwise the idle head, opened by the investigation.

The chase's leave in the duellist calls `lifecycle::return_to_idle` and
`lifecycle::restart_idle` on the brain.

## Open questions and how they were answered

- Which activity installs the idle driver? None: the apply-roles routine
  (REA decompile and listing) maps 1, 2, 3, 4, 5, 7 only; the driver's
  sole reference is an installer on the player body's behaviour gated by
  a byte whose only reference is that read (REA xrefs; the byte reads
  back as uninitialised data).
- Does the investigation ever walk? No: both pathfinders' disturbance
  slots return a constant 0 (vtable slot read through REA, the constant's
  three references are reads).
- Does the sleep tick write? Yes (listing): the dormant flag and the
  dormant condition.
- What does the wake test? The flag word equal to 0x10 and the best level
  above 0, then role 44 removal, the re-application flag and the knock-out
  with delay 0 (listing).
- Who calls the idle return? The attach, the level-leave reset and the
  chase route's leave (listing).
- Who sleeps? `exa-tool characters` over every level: one record holds
  role 44, `GdnShaperD` in `exanima67`.

No `docs/formats/` page changes: no data format is involved.
