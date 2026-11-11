## 1. Changes

- [x] 1.1 `PeriodicTimers<T>` with owner tags; tests `the_timer_table_rearms_from_the_frame_time`, `a_negative_reschedule_pushes_the_run_out_and_records_fire_in_order`
- [x] 1.2 `exa_game::frame_timers`: `FrameTimers`, `run_frame_timers` in `First`, the GUI timer as record 1, `EXA_TIMER_TRACE`; tests `the_gui_timer_is_the_first_record_and_ticks_every_100_ms`, `an_arrow_press_pushes_the_gui_timer_700_ms_out`, `a_one_shot_record_fires_once_when_its_owner_frees_it`
- [x] 1.3 `form_input::drive` gated on the GUI timer, the arrow pause; game screens, manual and Settings Controls page pass the table
- [x] 1.4 The arena's auto rematch as a one-shot 40-unit record

## 2. Checks

- [x] 2.1 Grid, exanima01 with the inventory open and a hinted record under the pointer: the tooltip shows on a GUI-timer frame (`timer table: record 1 (Gui) at 81615 ms`, then `tooltip: "Primary"`); the base build shows it too
- [x] 2.2 Lockstep traces (exanima01, 2 scenes each): the lane's runs match each other and the base build over the common ticks with body ids dropped
- [x] 2.3 fmt, clippy -D warnings for exa_game, the workspace tests on the worker
- [x] 2.4 docs/PERFORMANCE.md, docs/DEBUGGING.md, the exa_game README; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
