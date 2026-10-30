# Tasks

- [x] Trace the alarm return's caller chain (story hook, knock-out hook, story revive, world revive pass), the homeward callback, the sleep, the lookup and the pose reset with REA.
- [x] `exa_ai::golem`: dying-bit test, tone and condition writes, `alarm_return`; tests with the gate's numbers.
- [x] `exa_world::AlarmVolume::alarm_node`; installation check (every `exanima06` alarm item is named `Alarm_`), skipped without it.
- [x] `exa_game::flow::Step::AlarmReturn` in the revive before the save; `FlowSystems`; `StorySession`.
- [x] `exa_game::golem_alarm::return_from_alarm`; ECS tests (golems home and asleep, the player at (700, 6900) cm facing +x, the step becomes motion 0x1d; nothing without a guardian sent home or an armed alarm).
- [x] `Ragdoll::carried`, so a standing body is moved once.
- [x] `exarust --story-session`, `--give`, `--fell-player`; docs/DEBUGGING.md rows.
- [x] GPU runs on `exanima06`: wake, fight, knock-out, revive, golems home and statues again, the player at the post; with token 474 the gate lets the player through.
- [x] docs/PLAN.md and docs/COMPONENT-MAP.md are owned by the orchestrator.
- [x] fmt, clippy -D warnings and tests for the touched crates on the worker.
