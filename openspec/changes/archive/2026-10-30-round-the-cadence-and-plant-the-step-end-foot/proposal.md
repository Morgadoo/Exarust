# Proposal

## Phase

Phase 3, bodies: skeleton, physics, control (TMotileDynamics, ninth pass)
in docs/PLAN.md.

## Why

Round 39's motiledyn9 lane re-read three body rules against the shipped
executable and found them misported: the step end skipped every anchored
foot (the original passes over only feet anchored and not yet reached),
started its search from "no foot" instead of the original's
just-under-maximum sentinel with the first leg as fallback, and the body
tick's cadence target truncated where the original rounds half to even.

Evidence: the step-end routine's assembly (a `state & 0xc == 4` skip, the
sentinel 3.39999995e38 loaded once, a strict `COMISS` with a parity skip,
index 0 kept when nothing wins, `(state & 0xc0) | 2` and an anchor copy
when bit 8 is clear); the body tick's cadence block (`CVTSI2SS` of 1660,
`(1 − 0.25x)·1660` and `DIVSS` by 83 in single precision, a 64-bit
`CVTSS2SI` whose low word is stored); the pitch impulse (`v·v`, the
engine sign, then 2.0, and a double 0.95 decay).

## What Changes

- `exa_physics::body_balance::end_step`: the original's candidate test,
  sentinel and first-leg fallback.
- `exa_physics::motile::cadence_target`: ties-to-even 64-bit conversion
  keeping the low word (0.5 exertion gives 18; NaN gives 0).
- `exa_physics::motile::pitch_step`: the engine's sign (NaN keeps its
  sign) and the original's multiply order.

## Out of scope

- The status countdown that shares the cadence formula (no reader).
- The step end's soft-flag stamp, tail clears and zero-leg branch.
- Wiring the cadence target and pitch impulse into the running tick.
