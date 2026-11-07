## 1. Find
- [x] 1.1 List every navigation-unit and system-unit routine and its state (live, uncalled, unreachable, missing)
- [x] 1.2 REA: the game camera's preset records, the shots' placement, the MXCSR routine and its callers, the simulation thread's priority

## 2. Port
- [x] 2.1 The game camera preset and the chase camera's controller from it
- [x] 2.2 The shots through the placement routine, with a fast shot's distance
- [x] 2.3 The engine float mode on the main, simulation and task-pool threads
- [x] 2.4 Tests (preset numbers, shot placement, float mode per thread)

## 3. Check
- [x] 3.1 fmt, clippy -D warnings, tests, play build, openspec --strict
- [x] 3.2 Lockstep traces against main: exanima01, exanima06, arena duel
