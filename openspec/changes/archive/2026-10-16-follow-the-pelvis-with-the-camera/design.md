## Context

`chase_camera` runs in `Update`, after the fixed loop: it springs the orbit
and eases pitch and zoom in 17 ms ticks of real time, then places the eye
20 m from the player root's transform (the capsule's feet). Bodies are
drawn in `PostUpdate` (`draw_bodies`), each part between its last two ticks
by the fixed clock's overstep, before transforms propagate.

This change extends [`docs/PLAYER.md`](../../../docs/PLAYER.md) (Camera).
There is no `docs/formats/` page for behaviour of the camera; its evidence
is in the git-ignored notes (`context/re/master-movement.md` §11,
`player-control.md` §2).

## Goals / Non-Goals

**Goals:** the focus on the drawn pelvis, the original's tick and
interpolation rule, the camera's shake (a kick and the tick's ring-down)
for later sources.

**Non-Goals:** walls (tilt, fade), shake sources, other camera modes.

## Decisions

- **The pelvis as drawn.** The camera reads the player body's root part
  and asks it for its transform at the same overstep fraction
  `draw_bodies` will use in `PostUpdate`, so the focus is exactly where
  the pelvis is drawn in this frame; reading its `GlobalTransform` would
  give last frame's. The root part is the pelvis (bone 1), the node the
  original follows.
- **Ticks ahead of the frame.** The original's tick time is the end of its
  last tick and runs ahead of the frame; a tick runs once the frame passes
  its start. `ahead` keeps that lead (0–17 ms); t = 1 − ahead/17 is how far
  the frame is into the last tick. A stall runs nine ticks and drops the
  rest, as the original snaps its tick time to the frame.
- **What is blended.** The tick before the last's backward vector and zoom
  are kept (`before`); the frame lerps them toward the last tick's and
  renormalises the vector, as the original does; the pitch and yaw springs
  stay in their angles inside the tick.
- **Shake.** The camera keeps the eye's offset and the tick before's, as
  the original does. A source calls `kick(v)`, which adds v to the offset
  and takes it from the tick before's, as the original's shake call does
  (where the original's callers pick v is RB30, for later changes). Each
  tick carries the offset on: it keeps 0.96 of its change since the tick
  before and then 0.99 of the whole, a damped wobble of about a second
  (a 1 cm kick peaks near 14 cm and stops within 300 ticks); once the
  change squared is within 1e-7 cm² and the offset squared within
  1e-4 cm², both drop to zero. Frames lerp the two; the eye moves, the
  view direction does not, and nothing is added while the present offset
  is within 1e-5 cm² (the original tests that one, not the blend, so an
  ending shake drops out at once).

## Risks / Trade-offs

- The pelvis bobs 1–3 cm while walking (C8 traces); the view now bobs with
  it, as the original's does. A limp or fallen player keeps the camera on
  its pelvis, also as the original.

## Open questions (and how they were answered)

- Is the focus the pelvis or the model root? REA: start of play sets the
  follow object to the model root node `01Pelvis`, and the per-frame
  placement interpolates its body and reads the node (Q12 statically).
  RR6 (a retail walk with `exa_freecam`'s reader) would confirm the bob.
- Where is the shake added? REA: to the eye after the distance, lerped
  between the camera's two shake fields.
- Does the camera move the shake itself? REA (verification run): yes, its
  tick carries the offset on (0.96 of the change, then 0.99, zero once
  both are tiny), and the shake call adds to one field and takes from the
  other; the first implementation had sources set the offset outright and
  was corrected.
