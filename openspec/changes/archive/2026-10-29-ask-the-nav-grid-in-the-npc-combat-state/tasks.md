## 1. Queries

- [x] 1.1 Read the combat state's pathfinder calls, the combatant's cost routine and the AI tick's footprint lift and stamp in the assembly (REA/exe listing)
- [x] 1.2 Add `exa_ai::ground` (ground cost, line walk clamp, obstacle) with tests on synthetic grids, the abstract answers without one

## 2. The duellist

- [x] 2.1 Answer the duellist field's ground cost, line walk and sight through `ground`
- [x] 2.2 Lift the duellist's own footprint around its combat ticks and stamp it again

## 3. Docs and checks

- [x] 3.1 Update `crates/exa_ai/README.md` and the combat module's notes; docs/PLAN.md is updated by the integrator, no docs/formats/ page changes
- [x] 3.2 No corpus format is read, so no exa-tool corpus check applies (the arena run on the GPU uses EXANIMA_DIR)
- [x] 3.3 `cargo fmt`, clippy `-D warnings` and `cargo test -p exa_ai` (remote worker)
