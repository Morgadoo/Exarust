## 1. Audit

- [x] 1.1 Classify the class's 120 routines (live, uncalled, partial, missing, unreachable) against the crates
- [x] 1.2 Check the re-place, its helper and its callers, and the sole block, against the assembly

## 2. Port

- [x] 2.1 Keep the held item's place under its hand when its grip links are built
- [x] 2.2 Re-place held items on request: at rest at the grip, woken, the second-hand slack reset
- [x] 2.3 Ask for it after the player's ground placement and after a Long Door
- [x] 2.4 Keep the sole turn's side axis on the leg and use it for a moving body's planted foot
- [x] 2.5 Unit tests for both

## 3. Check

- [x] 3.1 No corpus format is read (no `cargo exa` check applies); GPU grid run logs the re-place in exanima01 and the arena
- [x] 3.2 Update the exa_physics and exa_game READMEs (PLAN.md is the integrator's)
- [x] 3.3 fmt, clippy -D warnings and tests for exa_physics and exa_game on the worker; play build
