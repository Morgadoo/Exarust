## 1. Find
- [x] 1.1 REA: the lift's take-out, each placement's state and countdown, the lock release
- [x] 1.2 REA: the "is the space free" query and its pair test

## 2. Port
- [x] 2.1 Hold the lifted body out of the simulation, place it whole each frame
- [x] 2.2 The lift's ray meets props with no stand-in
- [x] 2.3 The space query, wired to the marker overlap
- [x] 2.4 Tests

## 3. Check
- [x] 3.1 GPU: release 128 and a neighbour 0 -> 128 (main: 256); the lift ray meets a dropped cudgel; story level and duel traces unchanged
- [x] 3.2 fmt, clippy -D warnings, tests, play build, openspec --strict
