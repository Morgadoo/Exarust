## Why

A limp body in Exarust swings freely inside every joint's range: between
the origin pivot and the hard radius at the slack nothing resists the
joint. The original damps a limp joint inside its range. Each slack link
keeps a smoothed length that follows the link's length at the rate of its
setup smoothing, and every pass takes the two points back to the length it
had, so a change of length is resisted in proportion to how fast it
happens and a joint at rest stays where it stopped. Without it, the limp
character view comes to rest kneeling with its forehead on the floor, and
only one of the seven exanima01 corpses sleeps within a minute.

Exarust's limp retention, 0.2556 / 0.15 / 0.03 per second, is an
environment the original builds (its default × 0.6) but never gives to any
body. Every character, living or dead, uses the default environment:
0.426 / 0.25 / 0.05 per second. Living limbs never take the speed bands;
they keep the fast value × u², which is 1 for a human.

Phase 3 (Bodies), milestone M3 "standing man". This is change C4 of the
body-controller plan. It depends on C3 (done); Q8 is answered.

## What Changes

- A limp slack link inside its slack is a damper: with |d| its length and s
  its smoothing, L ← s·L + (1 − s)·|d|, and the correction is strength ×
  (|d| − L_old)/|d| × d, shared by the parts' mobilities. Beyond the slack
  it is the hard radius at full strength, and L becomes the slack.
- L starts at the posed distance when the body is built, and at the
  distance clamped to the slack when the body goes limp (`go_limp`, the
  original's death).
- A limp link is a pivot when its slack is at most 1.19e-7 cm, the
  original's limp test (0.01 cm stays the living one).
- Limp retention by the default environment: 0.426 / 0.25 / 0.05 per
  second, 0.98977 / 0.98344 / 0.96455 per tick. Each part takes its band
  from the farthest one of its particles moved in the last tick, strictly
  over 40/83 cm (fast) or 20/83 cm (slow), else resting.
- Living limbs keep the fast retention × u² whatever their speed: 0.98977
  per tick for a human (u = 1).
- Removed: Exarust's 0.2556 / 0.15 / 0.03, and the speed bands on living
  bodies.

Evidence:
- Reverse engineering of `Exanima.exe` 0.9.5: every writer of a body's
  environment, the environments' values and their per-tick conversion, the
  plain step's speed bands, the living limb step's retention, the plain
  link iteration's damper, and the callers of the smoothed-length reset
  (`context/re/body-q8-q9.md` §1–§6 and §10; `body-links.md` §6;
  `body-tick-order.md` §3–§4).
- Exarust runs through `gpu_run.sh`, against the C3 build, with a
  temporary sleep probe: `--limp character`, `play exanima01`,
  `--shove 60 play arena`, `--opponent play arena`.

Out of scope:
- the sleep countdown (S1): Exarust keeps its 1 cm/s for 2 s rule;
- the living retention's ×0.95, which the original applies in a character
  state set by dormant statues and golems that Exarust does not have, and
  u for other races or off balance (C7);
- per-particle gravity weights, the placement routine and the electricity
  effect, which also reset smoothed lengths;
- knocked-out bodies, which the original keeps living (C11, C23). In
  Exarust only death goes limp (a kill, a placed corpse, `--limp`);
  knockdowns fall with their muscles, so `go_limp` stands for death.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `physical-body`: limp joints damped inside their slack through smoothed
  lengths, set at build and reset when the body goes limp; velocity
  retention by the original's default environment, banded for limp parts
  and fast-only for living limbs.

## Impact

- `exa_physics` (`body.rs`: `Link::length`, `Link::limp`,
  `Link::reset_length`, `Ragdoll::new`, `Ragdoll::go_limp`, `integrate`,
  `iterate`, the retention helpers; tests in `body_joint_test.rs`,
  `body.rs`, and the thin-wall test in `body_contact_test.rs`)
- `exa_game` (`limp.rs`: the corpus test checks the shipped smoothing)
- `docs/formats/RSQ.md` (Physical body), `docs/PLAN.md`,
  `crates/exa_physics/README.md`
