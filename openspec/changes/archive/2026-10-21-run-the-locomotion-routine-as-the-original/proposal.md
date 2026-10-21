## Why

The movement decision, the turn and the alignment of the original's body
tick are in, but a moving body then runs the original's locomotion
routine, which Exarust only had in part: the run blend rose and fell
without its bleeds (√alignment·stance level while running, vertical
travel always), a standing body kept its run blend (the original lets it
fall 0.02 a tick), only the stop distance of the gait columns was kept,
the walk's clock was Exarust's own approximation in the sampler (√run in
every gait, the latch's encumbrance factor only on the sprint row, no
height factor, no alignment envelope, no stopping or unsupported clock),
the walk started a tick late from the behaviour with no start key, and
the drive was not computed at all. The leg routine the decision calls ran
one law for "moving" (so the run's 0.6 thigh, the backpedal's standing
gains and the collapse's skip were missing), and the stance factor read
this tick's run blend in single precision, never 1 for a dormant body.

Phase 3 (Bodies: skeleton, physics, control), milestone M3 "standing man";
the TMotileDynamics tick kernel lane (wave 2), after the salvage lane's
movement decision.

## What Changes

- The original's locomotion routine (`exa_physics::body::locomotion::locomote`,
  new `body_gait.rs`) in its order: run blend with every bleed and cap;
  the race's per-tick gait table as the loader scales it (`GaitRow`,
  `GaitTable`, the feral character's own sprint row) and the gait columns
  blended from it (thrust, push, stop distance, clock rate, boost); the
  walk's start key (2, 4, 5, 8, or none) or its resume within 600 ms of
  game time; the boost ramp; the drive (phase push, boost, damping;
  braking damping alone); the speed and its direction along the heading;
  the locomotion clock's step with support (alignment envelope, stopping)
  and without (the kept phase).
- The body tick's standing branch: the run blend falls 0.02 a tick, 0
  getting up (`stand`).
- The decision's stop branch resets every gait column to the walk row;
  the turn wraps its error and the yaw once as the original does and keeps
  the stance facing.
- The movement stage runs the routine for a moving body and the standing
  branch otherwise, starts the walk clip in the decision's tick (from any
  clip, with the start key's capture, or resumed), and keeps the clock
  for a resume; the sampler's clip clock takes the locomotion's step
  whatever the clip (replacing Exarust's own walking clock).
- Muscle gains: the leg routine by the leg mode (thigh 0.2 walking, 0.6
  running, 0.25·scale standing or backpedalling; the limit 0.4 or 0.5·S²,
  s ← 0.9s + 0.1t in double), skipped while collapsing; the stance factor
  by the leg mode from last tick's run blend in double precision, 1 when
  dormant or in the shock's stagger.
- `RaceMuscles` carries the race's per-tick gait rows (in place of the
  clip rates alone); the viewer scales them from `races.rdb`.
- Docs: formats/RSQ.md (Locomotion, Movement decision, Gains, clock
  table), PLAYER.md (Movement), the exa_physics README.

Evidence:
- REA on `Exanima.exe` 0.9.5: the locomotion routine's decompile and its
  instructions in `context/re/exe.asm` (double-precision constants, the
  processor's truncating fraction, MAXSS, the dot-product orders, the
  start-key compares), the body tick's move and stop branches (stance
  factor, leg routine calls, the standing clock branch), the leg routine,
  the gait loader (1/83 in single precision, its square, the built-in
  stops, the backpedal's negation), the character init's feral sprint
  row; constants read from the PE. Notes in
  `context/re/locomotion-kernel.md`.
- The per-tick turn countdown and its "fatigue" factor have one reader,
  which stores them where nothing reads (`context/re/master-body.md`
  §6.1): ported as nothing.

Out of scope:
- walking by the carry (C18): the drive is computed and kept, not added
  to the carry (the capsule still carries the body, so adding it would
  move it twice); the feet's lag factor it needs comes with the feet;
- the gait audio mode (footsteps), the exertion value and the head tilt it
  feeds (the head look's sampler slot), the dead jump branch, the start's
  callback and part flag, the held items' static-only collision flag;
- the golem suit's halving of the leg gains and the anchored stepping
  foot's limit (no golem suit, no original steps in Exarust);
- the stop branch's get-up start, standing-position follow and turn toward
  a fall are already other stages' (balance, the character frame).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `player-movement`: the locomotion request's stop distance from the gait
  columns; the body turn's single wrap and stance facing; the walk started
  by locomotion; the run blend, gait columns, walk start and drive.
- `motion-playback`: the locomotion clock (clip clocks, backwards walk,
  sprint clock).
- `physical-body`: the muscle gains by the leg mode.

## Impact

- `exa_physics`: new `body_gait.rs` (and its tests) under
  `body::locomotion`; `Locomotor` gains the gait columns, drive, boost,
  clock step, phase and resume clock, stance facing (loses the stop
  distance); `Footing` takes a gait table; `Bearing` gains the leg mode,
  stance run and full stance; `RaceMuscles` the gait rows;
  `Ragdoll::root_moved`
- `exa_game`: `stages::movement` (the routine, the walk start),
  `stages::sampler` and `limp` (the clock takes the step; Exarust's
  walking clock removed), `stages::posture` and `limp::bearing` (leg mode,
  stance run, full stance), `balance` (the stance facing on a fall turn)
- `exarust`: the viewer scales the race's gait rows
- `docs/formats/RSQ.md`, `docs/PLAYER.md`, `crates/exa_physics/README.md`
