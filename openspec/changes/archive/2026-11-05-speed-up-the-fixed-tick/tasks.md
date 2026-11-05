## 1. Measure

- [x] 1.1 Profile exanima01 and exanima05_sw on main through the grid (perf on the grid child, chrome trace)
- [x] 1.2 Read the original's step driver with REA: its own thread, one step at a time, backlog dropped past four steps

## 2. Fix

- [x] 2.1 `exa_physics::body::first_by_key`; the held-item systems and the limb-impact pass look holders up through it
- [x] 2.2 The weapon refresh buckets held items by holder; the limb-impact pass keeps only the touched bodies' part states
- [x] 2.3 The pair stage tests a sleeping part's cell before its pose (`Part::cell`)
- [x] 2.4 The bodies' sound pass frees loops by query membership instead of a set of every body
- [x] 2.5 Debug aids: grid `exe=`/`env:` scene words, `EXA_SWEEP_STATS`, `EXA_FIXED_EXECUTOR`
- [x] 2.6 Tests: the key lookup matches `find`; a part's cell is its pose's; the grid's scene words

## 3. Docs and checks

- [x] 3.1 exa_physics, exa_character, exa_combat, exa_game, exa_audio and exarust READMEs; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 3.2 fmt, clippy -D warnings, tests of the touched crates on the worker
- [x] 3.3 Before/after through one grid: exanima01, exanima05_sw, exanima06, arena, arenahub
