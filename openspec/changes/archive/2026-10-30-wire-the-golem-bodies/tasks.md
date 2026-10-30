# Tasks

- [x] Find the race gates on physical bodies (`human_frame` in the viewer's placed characters and the scene bodies).
- [x] Trace the character builder, the spawn's initial motion, the sentry state and the golem wake with REA.
- [x] Check the golem models' bones and races against the installation (`exa-tool chunks`, `races`, `clips Golem`).
- [x] `physical_frame`, the placed characters' muscles, mind and duellist; test.
- [x] `exa_ai::golem` start motion, sentry tick, sleep and wake scales; tests with the original's numbers.
- [x] `exa_game::golem_body` and the posture stage's statue rest; the wake's rest end and scale undo; ECS tests.
- [x] GPU runs: exanima06 statues stay at rest; walking into the gate wakes two golems, which enter combat, walk to the player and strike; exanima01 `--opponent` still duels.
- [x] docs/PLAN.md and docs/COMPONENT-MAP.md are owned by the orchestrator.
- [x] fmt, clippy -D warnings and tests for the touched crates on the worker.
