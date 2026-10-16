## 1. Stopgap

- [x] 1.1 `exa_physics::body` `climb` (`body_climb.rs`), labelled Exarust's own: `CLIMB_ALLOWANCE` 6 cm and `climbing_ground` (max of the ground and the floor less 6 cm)
- [x] 1.2 `Ragdoll::lift` reads each leg's ground through it while `Bearing::carried`, the floor being the body root's height; `Leg::ground`, the Height Query and the grounded test unchanged
- [x] 1.3 Doc comments: `body.rs`, `body_lift.rs`, `Bearing::carried`

## 2. Tests

- [x] 2.1 The rule: floors 2, 5, 6 cm over the ground leave it; 20 cm raises it to 14; higher ground kept
- [x] 2.2 Level ground: walking and standing with the controller 0, 2, 5 cm up keep the original's target every tick; perched 12 cm, 6 cm higher; a controller in the air, or a fallen body, the original's
- [x] 2.3 A striding walker carried up four 18 cm steps by a modelled capsule, stopping 3 s on the third: balance kept, pelvis 72 cm higher (± 3 cm), feet on the landing; with the original's target alone it loses its balance
- [x] 2.4 Corpus checks (skipped without `EXANIMA_DIR`): the shipped body tests, unchanged

## 3. Verification

- [x] 3.1 In game through `gpu_run.sh --gpu 4000` with `--trace`, against the C8 build: both exanima01 stair starts (falls counted), `--autowalk play arena` (pelvis and target unchanged), `--shove 60 play arena` (get-up), `--autowalk play exanima01` (no falls; the perched start), `character`; no validation errors
- [x] 3.2 Remove probes and dumps from the tracked tree

## 4. Documentation

- [x] 4.1 `docs/formats/RSQ.md` (Physical body: the target's pointer and the Exarust's own stairs entry), `docs/PLAYER.md`, `docs/PLAN.md` (Phase 3), `crates/exa_physics/README.md`
- [x] 4.2 `openspec validate --all --strict`
- [x] 4.3 `make check` (fmt, clippy -D warnings, tests; `play` profile)
