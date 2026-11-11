## 1. Changes

- [x] 1.1 `exa_combat::attack`: `Busy::breaking_tangle` refuses a free start; test `break_tangle_refuses_a_free_start`
- [x] 1.2 The player's request and the AI's readiness set it from the playing clip
- [x] 1.3 `EXA_FORCE_TANGLE`, the replaced clip in the player's attack log, the held-request debug line
- [x] 1.4 Docs: the dead main-side start sound, the manager frame's parked-body wake, the part transfer's original callers

## 2. Checks

- [x] 2.1 fmt, clippy -D warnings and tests of exa_combat, exa_ai, exa_anim and exa_game; the workspace test once (bridge audit)
- [x] 2.2 Grid: a forced tangle break holds the player's request 74 ticks; without it the attack starts at once
- [x] 2.3 Lockstep traces against `exarust.r190` over the common ticks; `determinism_grid.sh`
- [x] 2.4 Crate READMEs; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
