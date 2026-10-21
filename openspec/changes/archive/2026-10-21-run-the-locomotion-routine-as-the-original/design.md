## Context

The original's body tick, for a body with muscles: the movement decision
(move vector, leg mode, brake, gates, move or stop branch), the body turn
and the alignment, then — with the locomotion amount over 0 — its
locomotion routine, else the standing clock branch; then clip-end
handling and sampling. The salvage lane ported the decision, the turn
table and the alignment. This change ports the routine and the parts of
the two branches it touches. Formats page: `docs/formats/RSQ.md`
(Locomotion, Movement decision, Physical body gains, Motion playback).

**Locomotion routine** (instructions, not the pseudo-code, for the
arithmetic): v = the root origin's last-tick displacement. Running (leg
mode 1): with support run ← √align·run·level (single), cap 1 − enc·0.3
(0.4 with the latch, double), run + 0.05 (double), run = cap when cap <
run. Otherwise, without brake/stopping run − 0.015 (double), with them f =
MAXSS(0.2·(speed − 2) (sub single, × double), 0), f·(2 − f), run = f when
f < run; run = 0 when run < 0. Always run − |v_y|·0.005 (double), floor 0.
Columns: backwards the backpedal row; else row = jog, or with the latch,
level > 0.9 (double) and speed > 3.5 the sprint row (the character's when
its feral level > 0); clock = row·run + walk·(1 − run) running, row·√run +
(1 − √run)·walk otherwise (√run = 0 at or under 1.19e-7); with the latch ×
(1 − 0.1·enc) (double); push, boost, thrust and stop by run; thrust and
push × level. Clock × the clock factor; crouch > 0.1 (double) × (1 −
0.1·crouch) (double). Motion ≠ walk: boost ramp 0, capture, last ran +
600 < now → key (4 backwards; error < −1: 5 when run > 0.5 else 8; error >
1: unchanged, which after the sampler consumed the last is "none"; else
2), clock = keys[key − 1]; else clock = the kept clock. Walk playing:
ramp + 0.5/83 (single constant 0.0060241), cap 1. Ramp × align. Drive:
braking v × tone·(−0.2) (double factor); else P = (1.1 − |0.5 −
frac(2p + 1)|·0.6) (double) × (2 − align²); along = (h·v)·8 (x + y then
z), negated backwards; along < boost → P += (boost − along)·ramp; drive =
h·(amount·thrust·tone·P) + v·(tone·−0.4) (double factor). Speed: proj =
h·((v.x·h.x + v.z·h.z) + v.y·h.y), its length (square root above the
epsilon), direction proj/speed. Clock with support: p' = frac((p + 1.5)·
2), e = 1 − |0.5 − p'|·1.8 (double), k = 1 − (1 − align)², K = (1 − k)·e·4
+ k; stopping with speed not over 5 (NaN too): rate·speed·K; else K ×
((speed·0.8 + 1)·rate) (double); phase = −100. Without support: phase <
−1 → 0.85 − round(frac(p + 1))·0.5 (CVTSS2SI, nearest-even; ×0.5 single;
the subtraction double); base = (speed·0.8 + 0.9)·rate (double); u =
frac((1 + phase) − p); step = (2 − u)·(base·u). Clock + step, < 0 → +
the walk's last key; kept with the time.

frac is x − (float)CVTTSS2SI(x): truncation toward zero, so negative for
a negative x, `i32::MIN` out of range.

**Standing branch** (amount ≤ 0): getting up (condition 0x300) run = 0;
else run − 0.02 (double), floor 0.

**Move branch / stop branch**: stance factor target (double) run·0.3 +
0.7 with last tick's run (locomotion runs later), s + 0.01 (double) while
under it, clamped to it; stop: 1 for condition 0x600 or the state word
exactly 0x100, else the race's. Leg routine (mode = leg mode moving, 3
stopped): 0 → thigh 0.2, target 0.4; 1 → 0.6, 0.4; else 0.2 × (1.25·scale)
(double), 0.5·S² (single); skipped in condition 0x800; strength s ←
t·0.1 + s·0.9 (double). Stop branch: every gait column back to the walk
row.

**Turn**: error = heading yaw − yaw, +2π below −π, −2π above π; the
stance facing = heading when |error| ≤ π/2 (single), else the yaw +
sign(error)·π/2; the step's yaw likewise wrapped once.

## Goals / Non-Goals

**Goals:** the routine exactly, in its order and precision; the clock the
sampler takes is the routine's; the walk starts where the original starts
it; the leg routine and stance factor by the leg mode.

**Non-Goals:** walking by the carry (the drive is kept for C18); the
footsteps, head tilt, jump, golem suit and original steps.

## Decisions

- **v while the capsule carries the body.** The carry moves both particle
  sets, so the body's own displacement leaves the capsule's step out.
  Until C18, v across the ground is the capsule's commanded step (the
  decision's speed already reads it) and v up and down the pelvis
  origin's own displacement. This keeps the decision's speed and the
  routine's equal, as in the original.
- **The clip clock.** The routine's step is computed in the movement stage
  (where the original adds it) and the sampler's clock advance takes it
  when locomotion ran this tick, whatever the clip; Exarust's own walking
  branch is removed. The clock kept for a resume is the clock plus the
  step, wrapped once below zero, computed in the movement stage as the
  original does.
- **The walk start** uses the existing start (capture) and the stagger's
  `jump_in` (clock at the key before, capture meant for the key), which
  mirrors the original's start-key consumption by its sampler. "No key"
  starts at the clip's start (inferred: the original reads its clock from
  before the key list, the dynamic array's length bytes, a denormal of
  about 0).
- **Leg mode on the bearing.** The stance factor and the leg routine read
  the decision's leg mode (`Bearing::legs`), so the interim footwork's
  stepping, which the decision does not count as locomotion, takes the
  standing gains as in the original. A body without a locomotion request
  walks while its controller moves it. The stance target reads the run
  blend before this tick's locomotion (`Bearing::stance_run`, set at the
  posture stage).
- **The gait rows** live on `RaceMuscles` (per tick, as the loader scales
  them) since exa_physics cannot read `races.rdb`; the character's feral
  level and clock factor come from its stats each tick.

## Risks / Trade-offs

- The walk's clock now carries the alignment envelope and the latch's
  encumbrance factor on the jog row; with misaligned turning the walk
  cycle speeds and slows within a stride as the original's does. No
  retail capture was taken (the game was not launched).
- The run blend now bleeds while running by √align·level: a turning
  runner's blend settles lower (0.5 at an alignment of 0.81), so the
  capsule's run speed, which reads the blend, drops with it.

## Open Questions

- The phase's start value: Exarust starts it unset (−100); the original's
  body is zero-filled at creation, so its first unsupported tick may read
  0 instead. Answer: a retail watch of a body that spawns in the air.
- The meaning of the start's callback (+ its argument 4) and of part flag
  0x10000 the start clears: REA xrefs on the callback's writers.
