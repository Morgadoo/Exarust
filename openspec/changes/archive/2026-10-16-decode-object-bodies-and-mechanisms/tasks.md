## 1. Research

- [x] 1.1 Confirm the plan's rules in REA and the disassembly: the rigid-body chunk's reader and writer, the object factory's body class choice, the door, lever and push-surface section reads, the signal dispatch, relays and jobs, the link conversion, the cloth marker's handler and the scene flag it tests, the door sound strings; append the corrections to `context/re/master-physics.md` (git-ignored)
- [x] 1.2 Corpus statistics with a scratch probe (removed): bodies, links, sliders, classes, spheres and sound slots per pack; cloth markers per pack; door and operative sections and their sizes in `objdb.rdb` and every level's database

## 2. Formats

- [x] 2.1 `rigid_body`: `RigidBody::parse` (class, mobility, rest points, point masses, spheres, links with slider axis and travel, sound slots), total on malformed input
- [x] 2.2 `rfc::Node`: `rigid_body` (decoded body or its error), `cloth_marker`, `has_rigid_body`, `rigid`; the weapon length walk reads `has_rigid_body`
- [x] 2.3 `objdb::Item`: `door` (`Door`, `door` flags, `door_sound`), `operative` (`Operative`, `operative` classes and kinds, `return_ms`), `mechanism`

## 3. Tool

- [x] 3.1 `exa-tool bodies <pack> <entry>`, `exa-tool mechanisms [level]`, `exa-tool corpus bodies`

## 4. Tests

- [x] 4.1 Unit tests pinned to the notes: a door body (mobility 0.016, hinge pin at 242.9 cm, stop slack 134.04 cm, spheres 3.5 and 0.96 cm); sliders (plate 16 → 4 cm on (0, −4, 0), portcullis 28 900 → 170 cm, negative → 0); sound slots (`ChnAMov`, 0.5, 1) and unknown sub-chunks; malformed bodies (short, impossible counts, overruns, a slider without its axis, every prefix); the node's body, its error and the cloth marker; a portcullis door section (`0x254`, `doorf_slm`); lever (return 5000 ms), relay, inverted button and fire operative sections; the operative class deciding before the door section; short sections
- [x] 4.2 Corpus (`exa_formats`, skipped without `EXANIMA_DIR`): `Objects.rpk` 1 253 models, 1 163 with bodies, 1 230 bodies all decoded, 69 with links and 65 hung from the world, 11 slider bodies with 33 links whose travel is their axis length, the class-word counts, 2 sound slots, `door uw01a`, its knocker, `lever a01`, `pressplateu01`, `button a01`, `portcullis uwa`, `brazierhang01`, the largest sphere set (7 839); cloth markers (6 hair models with 2 each, 4 banners, the wraith); `objdb.rdb` 47 doors and 49 operatives (47, 6, 5 mechanisms); `exanima01` 43 doors and 72 operatives (39 + 1 levers, 11 + 1 push surfaces, 6 fire jobs, 8 relays, 6 keys; 43, 40, 12 mechanisms), the doors' linked keys, levers and relays, one dangling link
- [x] 4.3 `exa-tool corpus bodies` and `exa-tool mechanisms exanima01` against the installation

## 5. Verification

- [x] 5.1 No in-game run: the change adds data the game does not use yet; the weapon length walk keeps its answer (`has_rigid_body` is the old flag), covered by the weapon-record corpus test
- [x] 5.2 Remove probes and scratch files from the tracked tree

## 6. Documentation

- [x] 6.1 `docs/formats/RFC.md` (Rigid body, Cloth marker), `docs/formats/OTHER.md` (Door and operative sections), `docs/formats/README.md`, `docs/PLAN.md` (Phase 1 table, Phase 3), `crates/exa_formats/README.md`, `crates/exa_tool/README.md`
- [x] 6.2 `openspec validate --all --strict`
- [x] 6.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
