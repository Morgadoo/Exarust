# Tasks

- [x] Move the held-role list to `exa_formats::roles` and use it from `exa_ai` and `exa_script`; double-precision weight share.
- [x] Run role init scripts through `exa_script`'s interpreter; corpus test of every shipped init script (skipped without `EXANIMA_DIR`).
- [x] Mood tests, rounding, memory load and the event filing checked against the assembly; tests with hand-worked values.
- [x] `StartDialogue`, the event node's checks, `InZone`/`InLocale`, `WitnessRole`, scripted objects and the drain.
- [x] Corpus checks: every shipped script runs, Derrin's topics, the narrator, every dialogue link (skipped without the game).
- [x] Update `docs/formats/OTHER.md` and the crate READMEs. `docs/PLAN.md` is updated by the integrator.
- [x] `make check` equivalent on the worker: tests and clippy `-D warnings` for `exa_script`, `exa_ai`, `exa_formats`, `exa_game`.
