## 1. Research

- [x] 1.1 Confirm the plan's rules in REA and the disassembly: registration (asleep at countdown 0, material scaling, the frozen node flag, link mobility, placement at the matrix, articulated children and their pivot, the world pivot, the "no level" mask), the sector's node walk (which nodes register, static colliders), link conversion and the runtime link adder, mobility shares, the body chunk reader and set-up (gravity shares), the material constructor and loader (RB32), the level's prop reader; append the findings and corrections to `context/re/master-physics.md` (git-ignored)
- [x] 1.2 Corpus statistics with a scratch probe (removed): material divisors and sound prefixes of every body node, node flags of body nodes, prop flags of every level, body trees per class in `exanima01` and `arenao1`, placement scales, child bodies, props parented to props

## 2. Formats and assets

- [x] 2.1 `material::Material`: `physical_divisor`, `collision_sound`, `mobility_divisor`, `collision_sound_set`
- [x] 2.2 `level::Prop::STATIC`, `is_static`
- [x] 2.3 `exa_assets::ModelNode::rigid` (`NodeRigidBody`: the body and its first material's divisor, cached per material)

## 3. Solver

- [x] 3.1 `body.rs` seams: `Part::gravity` shares, bodies that skip movable level pieces, the fall-out flag below −50 m, `draw_bodies` leaving deep sleepers and never-simulated bodies alone
- [x] 3.2 `body_rigid.rs`: `RigidPart`, `RigidLink`, `gravity_shares`, `Ragdoll::rigid` (particles at the node matrix, mobility, friction 0.75, articulated children with stored links and an origin pivot shared by mobility, asleep at countdown 0)
- [x] 3.3 `contact::Level::movable`; the static build tags the meshes under a `BodyNode` as stand-ins and lists them (`prop::StandIns`)
- [x] 3.4 `prop.rs`: `PendingProp` registration after transform propagation, `PropBody`, stand-ins moved with their bodies, removal of fallen bodies

## 4. World

- [x] 4.1 `exa_world::bodies`: body trees, the plan (free / hung / mechanism / static prop / flat), registration of free trees; `spawn_model_nodes`; the level load registers its own model, props and placed objects; `LevelInfo::bodies`

## 5. Debug and tool

- [x] 5.1 `--shove-props <frame>`; `--shove` to character bodies only; the body overlay draws awake props only, the traces, the joint trace and the scripted pauses keep to characters; the load log counts bodies
- [x] 5.2 `exa-tool bodies` prints each body's material, divisor, registered mobility and impact sound set

## 6. Tests

- [x] 6.1 Unit tests pinned to the notes: material divisor and sound set (0 → 1, 3, `mtlcol`, `wdlcol`, `MtlM`); static props (`0x80000`, `0x180000`; not `0x100000`, `0x200000`); gravity shares (1; 2, 1, 0.5, 0.5; one tick falls 0.142352 cm × share); a registered body asleep at its node with friction 0.75; a sleeping prop stays put for 500 ticks; a shoved prop lands, slides ~6 cm and sleeps after 140 ticks; an articulated child (shares 0.7734 / 0.2266, pivot within 1 cm after falling); a prop meets only the level, a character part the stand-in; falling out after 455 ticks; flat rest points; body trees (sack, hung chain, body under a plain node under a body); a child's slider slack; registration at the node's world transform; stand-ins following their body; the running solver (asleep for 60 frames, a blow wakes it, removed below −50 m); stand-ins built by the static build; deep sleepers not redrawn
- [x] 6.2 Corpus (skipped without `EXANIMA_DIR`): divisors of every body node of `Objects.rpk` and `Objlib.rpk`, three loaded models' divisors (`exa_assets`); `exanima01` 4 591 / 5 000 / 122 / 198 / 492 and `arenao1` 17 / 17 / 3 / 0 / 68, its tavern table a level node marked static (`exa_world`)
- [x] 6.3 `exa-tool bodies Objlib "barrel ubm a01"` against the installation

## 7. Verification

- [x] 7.1 Light in-game runs on the RTX 4000 (`--no-gi --no-supersample --size 960x540`): `arenao1` registers 17 bodies; `--shove-props` shoves the 8 pieces of tableware on the tavern table (level geometry: its node is marked static), seven of which land on the table and one 0.8 m lower on the floor, all asleep again after 236 ticks; before/after screenshots. `exanima01` loads with 4 591 bodies registered, then runs out of GPU memory before a frame (about 1.1 GB free)
- [x] 7.2 Remove probes and scratch files from the tracked tree

## 8. Documentation

- [x] 8.1 `docs/formats/RFC.md` (Rigid body, Materials, Props), `docs/PLAN.md`, `docs/DEBUGGING.md`, `docs/ARCHITECTURE.md`, `crates/exa_physics/README.md`, `crates/exa_world/README.md`, `crates/exa_formats/README.md`, `crates/exa_assets/README.md`, `crates/exa_tool/README.md`
- [x] 8.2 `openspec validate --all --strict`
- [x] 8.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
