## Context

The original's body tick sets a clip rate in its posture block and adds
frames from it in its clock block, then samples the clip part by part in
one fixed pipeline. The posture block's branches each write the rate
(1.25 × base off balance, the get-up's, 2 × base for the kick and the
clinch, 1 × base for the combat idle, a weapon switch, a power and every
other clip; the attack routine its own), every branch jumps to the
block's end, and the end resets the rate to 1 × base while the current
motion is the stance: the reset reaches the off-balance branch too. The
clock block adds tone × rate × 0.6 above half tone getting up, the rate
for attacks, 0.6 × rate otherwise unless the condition has bit 0x80, then
runs the end test. The research notes, with addresses, are git-ignored:
`context/re/master-animation.md` §3, §7.1 and its M14 findings,
`master-plan.md` (B1, K.1, K.5.3).

The format page this change extends is
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Motion playback:
the clock tables, the sampler pipeline, bodies without a tick; the trace
columns).

## Goals / Non-Goals

**Goals:**
- the rate table and the advance as the binary has them, including the
  stance reset, the kick, attacks off balance and the still bit;
- the sampler as the original's per-part pipeline, one function a step,
  with identity slots the later changes fill;
- the end test, the rate and the progress exposed; the trace columns;
- one clock for every body: bodies without a tick count body ticks from
  the frames' time.

**Non-Goals:**
- filling any slot (M16, C13, C19, M8); the clinch; a kick input.

## Decisions

- **Rate then advance.** `clip_rate` mirrors the posture block (the
  stance test first, since it overrides every branch) and `clip_frames`
  the clock block; C10's and C11's numbers are unchanged (the get-up's
  first tick at 1.25 × base through `Pace::rose`).
- **Where the pipeline lives.** The steps that need the body's parts
  (the chains, the IK, the twist, the look, the pitch) are in the sampler
  stage, `exa_game::stages::sampler`, which owns the pipeline; `exa_anim`
  keeps the steps a body without parts also needs (the bracket, the keys
  blended, the hold, the kept targets) as separate functions so the stage
  can put its own between them. Each identity step takes and returns its
  value, so it is called in its place and names its change.
- **The chain moves to the sampler.** The carry stage composed each
  part's placement from the kept pose; the original's steps 5–6 do that
  in the sampler, so the sampler now hands the carry the parts. Same
  arithmetic, same inputs: the targets do not change.
- **Attacks off balance.** The swing routine does not run off balance,
  so the clip runs at the falls branch's rate (1.25 × base), as
  `RSQ.md` already said; the code ran the swing's own rate. Corrected to
  the binary.
- **The weak branch.** It writes no rate, so the original keeps last
  tick's; Exarust reads the base (the value it would have in its stance a
  tick before). Documented, not modelled.
- **Progress.** `Animator::progress` becomes the field the sampler writes
  (the original's `+0xa28`), reset by a start; `clip_progress(clip)`
  computes the same value from the clock for callers that read it before
  the sample.
- **The accumulator.** `Animator::owed` holds the frames' time under a
  tick; a frame runs floor(owed × 83) ticks, each adding 0.6 × `base`.
  The character view's figure keeps base 1.0 (no race is wired to it).

## Risks / Trade-offs

- The pose-only sampler no longer clamps a looping clip past its keys: a
  caller asking far past the last key gets the first key (the share is
  clamped). Its only caller samples the walk's first key.
- The trace's own columns gain three between `hold` and the ankles;
  scripts reading by name are unaffected.

## Open questions

- RB6/RB7 (the live clip base and rates on a human) stay open: the retail
  game was not running. The binary's init and rate block settle the
  values statically (the critic's K.1 and this change's reading); a live
  read is still worth taking (RR1, RR7).

## Port onto main (2026-10-20)

The change was finished on 2026-10-17 but left unmerged; it was ported
onto a main that had meanwhile gained the run overlay, the sprint clock,
the backwards walk and the crouch's clock factor
(`blend-the-run-over-the-walk`, `clock-the-sprint`, `crouch-the-player`).
Those rules are kept as they are. The run overlay, which main took inside
one sampling loop, becomes the pipeline's second step: `exa_anim::
overlay_toward` slerps each bone toward the Run clip at the bracket's keys
and share, after the keys are blended and before the hold, which is where
the original's sampler applies it; `limp::run_overlay` picks the clip and
weight, and the sampler stage passes them in. `Animator::sample_layered`
stays as the same steps in a row for callers without parts.
`Animator::clip_progress` stays beside the new `progress` field: both
compute (clock − blend) / last key, the field as the last sample wrote it.
