# Proposal

## Why

docs/PLAN.md, Phase 7 (game modes and content), arena paragraphs: the match
return to the hub dropped each body's health word, a lesson that crossed a
rank left the new rank's kit empty, a rival week recorded the first tallied
team's company, and the match experience award was an invented split. This
change carries a DeepSeek pass on the arena unit and its review.

## What Changes

- The live return (`arena_live::settle_match`) writes each company body's
  health word into the manager's and the roster's records
  (`arena_session::write_back_health`); company bodies carry the numbering
  pass's participant id (manager 0, roster slot `n − 1` as `n`), no longer
  one too high.
- `career::copy_rank_loadout`: a lesson that raises the level sum to an even
  sum of at most 10 copies kit `sum/2 − 1` into `sum/2`.
- `arena::resolve_tournament` records the champion team's company.
- `arena::{xp_pool, xp_share}` replace the old split; the rank curve squares
  the square.
- Review fixes: the scene draw leaves the player's team out only on a
  forfeit (mode 0x10), as before the pass; both weighted draws keep the low
  word of a 64-bit conversion; the experience base is converted once (no
  separate tier rounding) and the share's ratio is divided in double; the
  roster health gauge and the bracket side's double start angle the pass
  changed are restored.

## Evidence

The retail assembly of the scene draw (the team compare falls through to the
grouping and only an equal mode 0x10 skips), the bracket draw (span = total,
strict compare), the experience award (one multiply of tier factor and
template experience before CVTSS2SI with a 64-bit destination; CVTSI2SD
damage ratio narrowed by CVTSD2SS), the roster cell (the GUI bar helper takes
x, y, length, thickness, fill, fill), the tournament ring (CVTSI2SD and MULSD
6.0 before the narrowed half spread is subtracted in double), the rank-up
helper and the weekly verdict.

## Out of scope

- Crediting the experience shares live: no scene body keeps a damage
  counter yet.
- A live caller for the scene draw and the roster health gauge.
- The auto-rematch schedule, the single reset entry and the window builders.
