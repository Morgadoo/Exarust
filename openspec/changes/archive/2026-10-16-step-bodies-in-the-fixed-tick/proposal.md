## Why

Exarust steps its physical bodies once a frame, not in the fixed tick. Each
body kept its own time accumulator and ran up to four ticks in `Update`.
Clips were sampled once a frame, and the frame's pose change was spread over
that frame's ticks, each target stepping a share of the motion ahead. The
body was carried with the character controller's drawn (interpolated)
position. As a result the targets a body follows, and so the body itself,
depend on the frame rate: at 30 FPS the muscles chase poses three ticks
apart, at 144 FPS one tick apart, plus an extrapolation of up to a frame.

The original runs one world solver at 83 Hz. Each tick runs every
character's body tick first (the player last), and each body tick samples
the character's clip and places the pose on the character. Then every body
is integrated, solved in 20 iterations, tested against the level and rubbed
in 8 friction passes. The player's behaviour runs once per body tick. Every
later body-controller change (link muscles, balance, support, feet,
locomotion through the carry) is written per tick, so the tick has to come
first.

Phase 3 (Bodies), milestone M3 "standing man". This is change C1 of the
body-controller plan; it depends on nothing.

## What Changes

- Bodies tick in the fixed step (83 Hz) as one world, in the original's
  order:
  1. every living body's character tick, the player's last;
  2. the queued blows;
  3. integration of every awake body;
  4. 20 iterations over all bodies;
  5. level detection;
  6. 8 friction passes over all bodies;
  7. the post-step.
- A living physical body's clip clock advances by one tick, and its clip is
  sampled, inside its character tick. Its targets are the body root's world
  transform times the clip pose. The pose's motion over a tick is this
  tick's targets against last tick's. The body is carried with its root's
  motion since the last tick. Inside the fixed step the root's position is
  the controller's simulated one.
- Blows (`Shove`) are queued before each frame's ticks and dealt by the next
  tick, even when the frame that sends them runs none.
- Parts are drawn between where the tick before last and the last tick left
  them, each under its drawn parent, after whatever the frame did to the
  character or the body root (a turn, the teleport of a body getting up).
- `exarust --trace <file.csv>` writes the body state after every tick: the
  player's body, or without a player every simulated body. Lengths are in
  cm and the clip clock in frames, in the columns of the retail body probe.
  Values not simulated yet are left empty.
- Removed: `step_ragdolls` with its per-body accumulator and its 4-tick
  cap, the spreading of a frame's pose over its ticks, the per-frame aim,
  and the per-frame carry. Balance still runs once a frame, over the ticks
  that frame ran. No other behaviour changes.

Evidence: reverse engineering of `Exanima.exe` 0.9.5, namely the
simulation sector step (tick order), the character body tick (clip
sampling, pose placed in the world), and the player behaviour called from
the body tick. Plus runs of Exarust: per-tick traces, screenshots, and the
regression set of the body-controller plan, compared with the build before
this change.

Out of scope:
- the original's clip clocks (Exarust keeps 60 frames per second until it
  is pinned);
- clip blending;
- running balance, falls and get-ups inside the character tick (they stay
  per frame until balance is rewritten);
- AI behaviour inside the tick;
- contacts between bodies;
- the retail probe itself (it is research tooling and stays untracked).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `physical-body`: the tick order and where clips are sampled, blows
  between ticks, drawing parts between ticks, the per-tick trace; living
  bodies in motion, the fixed rate and smooth motion updated to match.

## Impact

- `exa_physics` (`body.rs`: `BodyPlugin`, `CharacterTicks`, `BodySystems`,
  `solve`, `ShoveQueue`, `BodyTicks`, `draw_bodies`; `lib.rs`: the
  controller keeps its simulated position in the transform during the tick)
- `exa_anim` (`TickedClips`, `Animator::advance`, `ClipPose`)
- `exa_game` (`limp.rs`: the character tick; new `trace.rs`; `balance.rs`
  ordering)
- `exarust` (`--trace`)
- `docs/formats/RSQ.md` (Physical body), `docs/ARCHITECTURE.md`,
  `docs/PLAN.md`, `docs/PLAYER.md`, `crates/exa_physics/README.md`
