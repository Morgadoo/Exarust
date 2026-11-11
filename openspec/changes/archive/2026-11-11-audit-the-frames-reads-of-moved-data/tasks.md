## 1. Changes

- [x] 1.1 `BridgeHalf::frame_absent` and the frame audit (`sim_bridge::audit`); tests `the_audit_names_a_frame_reader_of_a_moved_resource`, `the_audit_stops_a_debug_build_at_an_unallowed_reader`
- [x] 1.2 The debug weapon blood posted to the fixed step (`stain_debug_weapon`); test `the_debug_weapon_starts_bloody` checks the list's spot
- [x] 1.3 `BrainTickClock` as a snapshot with growing totals; the profiler takes differences

## 2. Checks

- [x] 2.1 Audit over exanima01, exanima04 (lockstep), exanima08 and the arena with the profiler, traces, digest and debug views: two readers before, none after
- [x] 2.2 `--weapon-blood 0.5` in the arena: spot 1 (before: spot 0); `--perf-log 5` on exanima01: brains time and runs (before: 0.00, 0.0 runs)
- [x] 2.3 Plain and sword duels and exanima01 lockstep traces identical to main
- [x] 2.4 exa_physics, exa_character, exa_ai and exarust READMEs; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.5 fmt, clippy -D warnings, tests of the touched crates and exa_game on the worker
