## 1. Placement

- [x] 1.1 Take each node transform record from the level's frame into its node's local transform at the placement (props and placed objects)
- [x] 1.2 Unit test: a wall ring's record ends with the model's own local transform

## 2. Navigation

- [x] 2.1 Mark spawned meshes with rigid bodies, stamped where the placement makes them level geometry
- [x] 2.2 Skip their points in the stamp and raise the floors from them before the cost passes
- [x] 2.3 Unit tests: a door leaf walks its doorway up to its top; level geometry stamps
- [x] 2.4 Corpus (`EXANIMA_DIR`): `shipped_levels` numbers for arena, arenao1 and exanima01, the stair room reached on the floors as built

## 3. Docs and checks

- [x] 3.1 Update docs/NAVIGATION.md, docs/formats/RFC.md and crates/exa_world/README.md (docs/PLAN.md is updated by the integrator)
- [x] 3.2 `make check` equivalent: fmt, clippy -D warnings, exa_world, exa_game, exa_ai, exa_tool, exa_physics tests
