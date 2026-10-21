# Tasks

- [x] Remove the duplicate stagger component; keep the swing's stall and the Force wall interrupt.
- [x] Rewrite `exa_combat::parry::run_parries` after the original's callers: the combat-idle entry and the parrying entry, tone and permission gates, continuation flag, stored motion and clip, counter decay, the ending stance.
- [x] Guard follows the parry and never replaces a shield; the hit path opens it.
- [x] Hand-worked tests through the real character tick (parry start, hold and end, gates, window edges, endings, a stalled swing).
- [x] Drop the duplicate `spheres` fields left by the rebase.
- [x] Crate READMEs and `docs/formats/RSQ.md`. `docs/PLAN.md` is updated by the integrator.
- [x] fmt, clippy -D warnings and tests for `exa_combat`, `exa_physics`, `exa_ai`, `exa_game` on the worker.
- [x] `openspec validate --all --strict`.
