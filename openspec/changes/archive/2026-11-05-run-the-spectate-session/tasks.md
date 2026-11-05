## 1. Audit

- [x] 1.1 List both components' routines and mark them live, uncalled or missing
- [x] 1.2 Trace the spectate hooks, their callers and the interaction's spectate branch; find closest_to_cursor's callers

## 2. Port

- [x] 2.1 The spectate session, its follow rule and the camera's focus
- [x] 2.2 The behaviour tick's steering stopped while spectating; `--spectate`
- [x] 2.3 Tests

## 3. Check

- [x] 3.1 No corpus format is read (no `cargo exa` check applies); GPU grid: arena spectate follow
- [x] 3.2 Update the exa_game README and docs/re/playerctl-map.md (PLAN.md is the integrator's)
- [x] 3.3 fmt, clippy -D warnings and tests on the worker; play build
