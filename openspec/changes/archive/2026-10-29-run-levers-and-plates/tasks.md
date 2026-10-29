# Tasks

- [x] Read the lever init and step, the push-surface init and step, the door init's sliding branch, the dispatch's class 5 branch, the object interaction and the wheel press and drag handler in the instructions (REA)
- [x] Add the mechanism environment to `Ragdoll`
- [x] Run the lever's set-up and step on a registered arm (`exa_physics::body::lever_body`), with synthetic-lever tests
- [x] Run the plate step on a registered push surface (`body::plate_body`), with synthetic-plate tests
- [x] Run the door's sliding step on a registered sliding leaf (`body::slide_body`), behind the loader's flag
- [x] Keep a root slider link's travel as its slack
- [x] Register lever arms and push surfaces from placed objects (`exa_world::bodies`), `MechanismBodies`, `--static-mechanisms` and `--sliding-doors`
- [x] Route mechanism signals to the door bodies or push surface under the target
- [x] Turn a lever with the wheel drag and push a clickable surface with the double click
- [x] Verify against the installation: `exanima01`'s levers and plates rest, a turned lever signals its target (skipped without `EXANIMA_DIR`)
- [x] Update docs/PLAYER.md, docs/formats/RFC.md and the crate READMEs (docs/PLAN.md is updated by the integrator)
- [x] `cargo fmt`, clippy -D warnings and the affected crates' tests
