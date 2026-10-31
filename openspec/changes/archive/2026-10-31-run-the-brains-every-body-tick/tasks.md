# Tasks

- [x] Trace the physics thread's step, the sector step's character body ticks and the body tick's behaviour call in REA.
- [x] `exa_ai::BrainTick` in the fixed step (after the presence check and the daze's stance, in `exa_physics::body::TickBehaviour`, before the posture, the controllers, the pre-step and the character ticks) running the brain chain (`exa_ai::BrainChain`, single-threaded); `exa_physics::CharacterControllers`.
- [x] Order the fixed step's systems that share data with the brains against them (`--schedule-ambiguities`: none left); the profiler's `brains` line (`exa_ai::BrainTickClock`).
- [x] `pass_turn`: the counter is the step counter this tick's brains read; test.
- [x] Move the game's AI-tick parts into the brains' tick (follow and search, sentry, encounter scripts, the duellists' power system); the cast tests run the power system on its own.
- [x] No corpus format involved (no exa-tool check); crate READMEs; no docs/formats or docs/PLAN.md change (the main session owns the plan).
- [x] GPU check: the arena brain duellist engages and fights, exanima06's golems wake and fight, passes per body tick in the debug log, a `--fixed-frames` run against a normal one, the Fixed and Update times before and after.
- [x] fmt, clippy -D warnings and tests for `exa_ai`, `exa_physics` and `exa_game`, and the play build, on the worker.
