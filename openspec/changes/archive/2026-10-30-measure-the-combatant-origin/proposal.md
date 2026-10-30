# Proposal

## Why

A further pass on the behaviour base (`TCombatant`) published the
fighting estimate at combat entry, measured each combat place from the
opponent's own origin, weighed a third party's threat by its locomotion
and in-combat marks, lowered courage in double precision, added the
removal from the scene and matched the head tracker's constants. A review
against the instruction listings found five rules wrong: the estimate's
level part was built in single precision; the thrust approach scale and
the stepping origin read the observer's skill where the original reads the
scored combatant's own; the moving origin leaned by the body size instead
of the body's locomotion speed; the arc publisher swapped its look (the
body facing) with the yaw its edges come from (the head look); and the
head tracker's wrap compared with the float pi and divided the flat look
where the original scales by the reciprocal length.

## What Changes

- The fighting estimate (`exa_ai::estimate`), one implementation shared
  with `exa_game`, published by the duellist at combat entry.
- The measured origin (`exa_ai::place::measured_origin`) and the per-place
  score (`score_place_at`), fed by the duellist with the target's
  locomotion amount, speed and its own skill.
- The thrust approach scale with the opponent's own skill.
- The third-party threat multiplier by locomotion, body movement bit 2 and
  the in-combat mark; courage lowered in double precision; removal from
  the scene with the forget broadcast as a tested function.
- The arc's look from the body facing; the head tracker's wrap, step,
  pitch floor and flat look as the listing has them.

## Impact

- Evidence: REA/Ghidra listings of `Exanima.exe` 0.9.5: the experience
  system's estimate, the rating refresh, the position score's origin and
  thrust branch, the perception pass's threat multiplier, the hit rule,
  the removal, the arc publisher and both head-tracking variants, read at
  the instruction level for compares, sum order and precision. No game
  data is read.
- Out of scope: the target's step flag for the stepping origin (the
  duellist does not take that branch), the head tracker running on
  characters, the player's behaviour skill (0), the scene despawn calling
  the removal, the mind-power calls, and the combat state's full candidate
  search.

## Phase

Phase 6 (AI) in `docs/PLAN.md`, the combat AI behind milestone M4 "first
duel" through the duellist.
