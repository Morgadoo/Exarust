# Tasks

- [x] Check every dynamics-volume routine, the rigid body's translate, place-at-matrix and Verlet step, and their callees against the instruction listings: constants, comparisons, rounding modes, precision, NaN branches, loop bounds and output order.
- [x] Run the fit on the voxeliser's map in the builders' order.
- [x] Correct the grid (stored box, short axes rounded up, wide-query radii in double), the mesh queries (the original's triangle record, nearest point and ray test), the solid fill's ray lengths, the summary's empty grid, the extraction (count-bounded candidates, the largest kept, the original's windows and cell sizes, the share and mirror tables) and the weighing.
- [x] Correct the translate (weights, strain only with the restore flag), place-at-matrix (contacts and strain kept, the original's point transform) and the settling tick (no velocity recorded).
- [x] Tests: hand-worked grid counts, the face tests at their radius, one-sided rays, the tree against every face, a box's volume and central sphere, the pruning order on a rod, the corner masks, the share and mirror tables, the fallback slab, the point masses, the translate's weights and strain, the settling tick's velocity.
- [x] Real-installation check: every shipped `Objlib` body node fitted and compared bit for bit with its stored spheres (`exa_world::bodies`), skipped without `EXANIMA_DIR`.
- [x] Update `crates/exa_physics/README.md`; `docs/PLAN.md` is updated by the main session.
- [x] `make check` equivalent: `cargo fmt --all`, clippy `-D warnings` and tests for `exa_physics` and `exa_world`.
- [x] `openspec validate --all --strict`
