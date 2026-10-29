## 1. Flight

- [x] 1.1 Plan the flight through `route::plan` on the brain's turn, follow only routes over 150 cm, remember the exit's room for a followed route
- [x] 1.2 Test the routed flight in a Bevy app with a synthetic grid

## 2. Idle return

- [x] 2.1 Remove the restart and stand reset stubs; keep the idle return with its test

## 3. Docs and checks

- [x] 3.1 Update crates/exa_ai/README.md (docs/formats/ unchanged; docs/PLAN.md is the integrator's)
- [x] 3.2 Real-installation check: none needed, no format read (EXANIMA_DIR checks skip unchanged)
- [x] 3.3 fmt, clippy -D warnings and tests for exa_ai, exa_game, exa_combat
