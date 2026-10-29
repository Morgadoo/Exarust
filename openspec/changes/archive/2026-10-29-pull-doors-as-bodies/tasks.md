# Tasks

- [x] Read the hand's door press and drag handler, the link-adding routine, the door init and hinged step in the instructions (REA)
- [x] Bucket a dense part's spheres for the pair fallback, tested equal to the plain loop
- [x] Run the door's set-up and step on a registered hinged leaf (`exa_physics::body::door_body`), with synthetic-leaf tests
- [x] Register hinged leaves from placed doors (`exa_world::bodies`, `Placement::Door`) and the `--static-doors` option
- [x] Name a leaf a hinge and pull it with the door link in the hand; let go on any release
- [x] Verify against the installation: `exanima01`'s 109 leaves rest shut and one opens under a pull (skipped without `EXANIMA_DIR`)
- [x] Update docs/PLAYER.md and the crate READMEs (docs/PLAN.md is updated by the integrator)
- [x] `cargo fmt`, clippy -D warnings and the affected crates' tests
