# Tasks

- [x] Port the three shape builders, the sphere sort and the setup's gravity shares over `volume::fit`.
- [x] Check every builder rule against the listing: loop counts, the side flag's tests (distance or sign), double-precision clamp, NaN branches, the factory record's bytes.
- [x] Port the dynamics world unit's rules as records and check them against the listing: grid move direction and strips, cell rounding, offsets, backlog drop, driver wait, purges, update order, placement query masks.
- [x] Register hung prop trees with world links anchored in their parent node.
- [x] Tests: tetrahedron for every axis order and the side flip, mirrored root order and growth, gravity shares, sphere sort ties, grid shifts in four directions, cells, purges, update order, query filters, particle matrix.
- [x] Real-installation check: `exa_world::bodies` corpus test counts the registered and hung trees of `exanima01` and `arenao1`, skipped without `EXANIMA_DIR`.
- [x] Update `docs/formats/RFC.md` and the crate READMEs; `docs/PLAN.md` is updated by the main session.
- [x] `make check` equivalent: `cargo fmt --all`, clippy `-D warnings` and tests for `exa_physics`, `exa_world`.
- [x] `openspec validate --all --strict`
