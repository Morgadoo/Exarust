# Tasks

## 1. The gates

- [x] 1.1 The overlay weight reads the sprint latch alone: at level 0.9 or 3.5 cm a tick a full blend with the latch meets the run bone; without the latch it is 0.7 of the way, in double precision.
- [x] 1.2 The stop distance takes the sprint row only above a stance level of 0.9: 1.5 m at a half blend above it, 1.25 m at 0.9.

## 2. Docs and check

- [x] 2.1 Update `docs/PLAYER.md`, `docs/formats/RSQ.md` (the pipeline table's overlay row too) and the `exa_game` README. `docs/PLAN.md` is the integrator's.
- [x] 2.2 Corpus, skipped without a format change. Remote tests and clippy for `exa_physics`, `exa_game` and their dependents, and `openspec validate --all --strict`, pass.
