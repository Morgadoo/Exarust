# Tasks

- [x] Trace the game loop's pre-tick hooks and step counter, the AI rotation hook, the AI tick's presence step and the integration's present branch in REA.
- [x] `exa_ai::pass_turn`: the turn resource, its rotation system and the claim; tests (three brains in turn, a lone brain every other tick, a newcomer, a brain that does not claim, the wrapping wait, forget).
- [x] Gate `perceive_all` and the target pass in `ai_tick` on the turn; `Npc::tick_between_passes`; the integration's present flag from the presence; the duellist's entry and leave on the pass's tick; tests (one brain a tick and the hostile still engages; a dormant brain does not wait).
- [x] Check exanima06's start against older binaries (no regression).
- [x] exa_ai README; no docs/formats or docs/PLAN.md change (the main session owns the plan).
- [x] GPU check: exanima06 golems wake and fight, the arena brain duellist engages, the per-tick pick in the debug log, the frame's schedule times before and after.
- [x] fmt, clippy -D warnings and tests for `exa_ai` and `exa_game`, and the play build, on the worker.
