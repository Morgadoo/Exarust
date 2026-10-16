## Context

The original's simulation sector step runs once per 83 Hz tick. First it
runs every character's body tick, from the last registered character down
to the first, which is the player, so the player is ticked last. Then it
steps every awake body, runs 20 constraint iterations, clears the contacts,
detects body–level contacts, runs 8 friction passes and post-steps every
body. Inside a character's body tick, the clip sampler builds each part's
animated transform in character space. The body tick then multiplies it by
the character frame into the world, before any body is integrated. The
player's behaviour (input steering) is called from the body tick, once per
tick. The research notes, with addresses, are git-ignored:
`context/re/body-tick-order.md` (§1 order, §8 body tick),
`context/re/player-control.md` (§1.6) and
`context/re/body-controller-plan.md` (C1).

Before this change, Exarust's `step_ragdolls` ran in `Update` with a time
accumulator per body and at most 4 ticks a frame. `aim_bodies` read the
frame's clip pose and spread its motion over the frame's ticks (`aimed`,
`target_steps`). It also carried the body by the drawn, interpolated
controller position once a frame. `steer_player` already ran once per fixed
tick, in `FixedPreUpdate`.

The format page this change extends is
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Physical body: how
a tick runs, and the per-tick trace).

## Goals / Non-Goals

**Goals:**
- the original's tick order, with bodies ticking in the fixed step as one
  world;
- clips sampled every tick, and targets equal to the root's world transform
  times the clip pose;
- particle positions after N ticks independent of the frame rate;
- blows that are never lost between ticks;
- parts drawn smoothly between ticks;
- a per-tick trace with the columns the retail probe (T0 of the plan) will
  write.

**Non-Goals:**
- changing what a tick computes (drive, gravity compensation, links,
  contacts, sleep and balance rules all stay);
- the original's clip clocks and blends;
- moving balance, AI or audio into the tick.

## Decisions

- **Two sets in `FixedUpdate`.** `CharacterTicks` comes before
  `BodySystems`, chained, and both come after the character controllers
  (`register_characters → move_characters → step_world`). `exa_game::limp`
  adds the character tick to `CharacterTicks`. `exa_physics` adds the
  solver, `step_bodies`, to `BodySystems`. `BodyPlugin` holds the body
  systems so that tests can run the real schedule without rapier's
  controllers or meshes.
- **One world solver.** `solve(&mut [&mut Ragdoll], level)` runs each phase
  over every awake body before the next phase starts: integration (inertia,
  gravity, the muscles' pull), 20 iterations, contact reset and detection,
  8 friction passes, then the post-step (where the tick left each part, the
  last two tick ends kept for drawing, the sleep count, the frame's tick
  count). Without contacts between bodies this gives every body the same
  result as ticking it alone. `Ragdoll::tick` is `solve` with one body.
- **Player last.** The character tick sorts the living physical bodies so
  that the one carrying `PlayerBody` comes last. The others keep query
  order. Nothing in this change depends on that order, but later changes do
  (balance, pins and feet read shared state).
- **Clips advanced by the tick.** A body made physical gets `TickedClips`
  next to its `Ragdoll`. `play_clips` skips such bodies, and the character
  tick advances the clock by 1/83 s at 60 frames a second (the clock is
  kept as it is, Q7) through the same `Animator::advance` that
  `play_clips` uses. A clip that ends into its follow-up gives the
  follow-up's first pose in the same tick, where a frame without a pose
  would leave the body's targets stale. Going limp removes the marker
  together with the animator.
- **Targets = root × clip pose.** For each part, the pose is composed from
  the part up to the body root. Body parts use their rest transform with
  the clip's rotation (the pelvis also takes the clip's root position, in
  place and scaled by stature); other nodes use their current transform.
  The result is multiplied by the root's world transform. The drive keeps
  this tick's targets and last tick's (`before`). The carry moves last
  tick's targets with the body, so the pose's own motion over a tick is the
  difference between the two. A tick without a new pose (`hold`) keeps the
  targets, still carried. A fall or a get-up clears both lists, so the next
  aim is a new start and does not count as motion.
- **The controller's simulated position during the tick.** `move_characters`
  writes the position it simulated into the transform. `show_characters`
  overwrites it with the drawn position after the fixed loop. A transform
  that differs from the drawn one when the next tick starts is still a
  teleport. So within the tick the body is carried by the root's true
  motion, as the original's character frame is.
- **Drawing.** Each part keeps where the tick before last and the last tick
  left it, untouched by the carry. `draw_bodies` interpolates each part
  between the two by the overstep fraction (lerp the place, slerp the
  turn). It writes the local transform under the drawn parent part, or
  under the node's parent as the frame left it. Each world transform is
  interpolated exactly once, so the controller and the parts are not
  interpolated twice. It runs in `PostUpdate`, before transforms
  propagate, after every frame system that moves a character or a body
  root: `show_characters` after the fixed loop, and in `Update` the AI's
  and combat's turns and a get-up's teleport. Drawn any earlier, a part
  would follow such a move for a frame before its tick does. A sleeping
  body draws its last pose.
- **Blows through a queue.** `begin_frame`, which runs before the fixed
  loop, moves every `Shove` message into `ShoveQueue` and resets the
  frame's tick counts. The solver deals the queue before integrating, the
  slot where the original's queued weapon impact acts: its post-step,
  before the next tick. The message API stays for the senders.
- **Balance stays per frame.** `keep_balance` and `get_up` run in `Update`
  over `Ragdoll::ticks`, the ticks the body ran since the frame began.
  This keeps their rules unchanged until balance is rewritten (C6).
- **The trace.** `BodyTrace` (opened by `exarust --trace`) writes one row
  per traced body after every solver tick: the player's, or every body
  without a player. Lengths are in cm and the clip clock in frames, the
  probe's units. The columns are listed in `docs/formats/RSQ.md`. Columns
  for quantities Exarust does not simulate yet (target pelvis height,
  stance factor, tone target, support, foot contacts) stay empty and fill
  as later changes add them. Exarust's frame height is the body root's
  height (its feet), where the original's frame sits at the pelvis height.
  The trace file is flushed every second and on exit.

## Risks / Trade-offs

- The order of the capsule move against the carry inside one tick: the
  controller moves first, then the character tick carries the body by the
  root's move. The original has no capsule, so this is Exarust's own order
  until locomotion moves to the carry (C18).
- The character tick walks every part's ancestry every tick (18 parts, 39
  characters in `exanima01`). That costs more than once a frame, but is
  small next to the solver.
- At 83 Hz the drive's goal motion is one tick of clip motion, where before
  it was a frame's motion spread with an extrapolation. A stagger can
  therefore come out differently. Shoved bodies in `exanima01` fell in the
  same numbers as before (32 and 32, or 33 for the old build in one run of
  two).
- The trace writes about 3 KB per body per tick. In play mode only the
  player's body is traced.

## Open Questions

| question | how it will be answered |
|---|---|
| Is the clip clock of physical bodies 60 frames a second or the original's rates (Q7)? | REA of the body tick's clock block, and the clock column of the probe (T0) against `--trace`, idle and walking; answered inside the clip-blend change (C10) |
| Where do the original's world targets sit vertically (Q1)? | the probe reading the pelvis height, the clip root height, the frame height and the pelvis's animated matrix while idle, against the same `--trace` columns; answered inside C8 |
| Does anything in the original depend on the player being ticked last within one tick? | the probe, once balance and pins move into the tick; nothing in this change depends on it |
