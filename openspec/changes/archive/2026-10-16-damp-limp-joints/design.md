## Context

Registration gives every body the simulation's default environment when it
has none, and nothing gives a character part another: living or dead, a
character keeps 0.426 / 0.25 / 0.05 of its velocity per second, converted
to per tick as r^(1/83). The plain step, which runs a dead character's
parts, picks a part's band from the largest squared displacement of its
four particles over the last tick, strictly above (40/83)² or (20/83)² cm²,
and adds (x_prev − x_prevprev)·k. The living limb step always takes the
fast value, × 0.95 in one character state, × u² (u = (1 − b)(1 − r) + r,
with r the race's limb damping ratio, 1 for humans). The 0.6× environment
Exarust had copied is built once in a class init and never assigned.

The plain link iteration damps a slack link inside its slack through its
smoothed length L: skipped when ||d| − L| ≤ 1.19e-7 cm, else old = L,
L ← L·s + (1 − s)·|d|, correction strength·(|d| − old)/|d|·d; beyond the
slack strength·(|d| − slack)/|d|·d with L = slack. L is set when the body
is registered, from the starting pose, and reset to |d| clamped to the
slack by death, the placement routine and one power. The living iteration
never touches L. The research notes, with addresses, are git-ignored:
`context/re/body-q8-q9.md` (§1–§6, §10), `body-links.md` (§6) and
`body-tick-order.md` (§3–§4).

The format page this change extends is
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Physical body).

## Goals / Non-Goals

**Goals:**
- the limp slack damper with smoothed lengths, set at build and reset when
  the body goes limp;
- the default environment's retention, banded per part for limp bodies;
- fast-only retention for living limbs, so they do not take the bands.

**Non-Goals:**
- the sleep countdown (S1); the living ×0.95 state and u off balance or for
  other races (C7); particle gravity weights; placement and power resets.

## Decisions

- **L on the link.** `Link::length` (metres) holds L. `Ragdoll::new`
  sets it from the posed distance of every slack link, which is the
  original's registration value (the node's starting-pose matrix maps the
  self point). `Link::limp` takes `&mut self`, so `iterate` walks a part's
  links mutably; the living branch reads and writes nothing new.
- **Reset in `go_limp`.** In the original a character switches to the
  plain iteration only when its muscles flag is cleared, which happens at
  collapse (which calls death at once) and at corpse spawn (registration
  sets L). So going limp is death in practice, and `Ragdoll::go_limp`
  resets L as death does, with the leg limit strengths.
- **Limp pivot threshold.** The limp test is slack ≤ 1.19e-7 cm, as the
  plain iteration's; the living one stays 0.01 cm. No shipped slack lies
  between, so nothing changes for the human body.
- **Retention helpers.** `per_tick` converts a per-second retention (0
  for r ≤ 0); `limp_retention` takes the farthest displacement in cm per
  tick and compares it with 40/83 and 20/83 strictly; `living_retention`
  is the fast value × u². Exarust keeps its 25 m/s particle guard.
- **u for humans.** Living bodies use u = 1 (`HUMAN_LIMB_DAMPING`): the
  human race's ratio of 1 makes u 1 at any balance. Other races and the
  balance term wait for C7, with the ×0.95 state.
- **Units.** L and slacks are metres; the band thresholds and the 1.19e-7
  cm tolerances are the original's cm values, converted once.
- **Thin-wall test.** Its 20 m/s ball ended its fourth tick behind the
  wall under the old retention too (the push goes mostly to one particle,
  so the centre moves about half of it) and passed only because it drifted
  back out through the wall's back. The test now uses 15 m/s and checks
  that the centre stays in front every tick.

## Open questions

| question | how it is answered |
|---|---|
| Who sets a body's environment, and to what (Q8) | Answered by REA: every writer of the environment pointer; characters always use the default (`body-q8-q9.md` §1–§3). |
| When smoothed lengths are set | Answered by REA: registration from the starting pose; reset by death, placement and one power (§5). |
| How much the damper damps in a whole body | Synthetic test: the point weights spread each pass, so a swinging hip keeps 0.8 of its swing a tick, not the 0.20 of a link closing in full; in game, R5 and R6 against C3. A retail capture (T0 probe) can confirm. |
| The ×0.95 state of living limbs | Static: set with dormant statues and golems; Exarust has no such state; left for C7. |

## Risks / Trade-offs

- [Limp bodies over-damped] → not seen: a killed duellist's pelvis reaches
  the floor in 0.72 s, as in C3, and R6 lies flat instead of kneeling.
- [Weaker resting retention (0.96455 against 0.95863 per tick)] → resting
  parts keep more velocity, and a joint under load sags slowly through its
  damper instead of dropping onto its limit: four exanima01 corpses and a
  fresh corpse keep a few cm/s of jitter, above Exarust's 1 cm/s sleep
  rule (with the old retention 5 of 7 corpses sleep by 41 s). The
  original stops this with its sleep countdown (below 20 cm/s), which is
  S1; until then some corpses stay awake.
- [Living bodies keep more velocity when slow] → living limbs keep 0.98977
  where they kept down to 0.95863; R3's fall and get-up keep their timing.
