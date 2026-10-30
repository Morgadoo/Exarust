# Tasks

- [x] Trace the original's behaviour calls in the damage routine and the hit chain (base count, combatant base, motile AI) in REA.
- [x] Write the blow notice at the three points in `exa_combat`, the wound credit with it; tested.
- [x] React in `exa_ai::struck`: courage, swing mark, lesson, flags word 3, concern with the threat estimate; tested with the original's numbers.
- [x] Apply the player's lesson in `exa_game`; tested.
- [x] `--opponent-brain` debug flag; GPU duel runs with and without it.
- [x] No docs/formats page changes (no data format); docs/PLAN.md is owned by the orchestrator.
- [x] fmt, clippy -D warnings and tests for `exa_combat`, `exa_ai`, `exa_game`, `exarust` on the worker.
