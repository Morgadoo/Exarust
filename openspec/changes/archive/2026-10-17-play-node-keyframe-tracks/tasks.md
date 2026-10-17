## 1. Decoding

- [x] 1.1 `exa_formats::key_track`: `KeyTrack::parse` (skipped word, mode word to `PlayMode` with unknown words free, duration, last index, keys of time and position in metres; truncated or overflowing counts are errors, trailing bytes ignored)
- [x] 1.2 `exa_formats::rfc`: sub-chunks after every stored node's body (a light's 40 B, +8 spot, +16 under body flags `0x30000`; a voxel light's 32 B; kind `0x3D08`'s 8 B; none for plain nodes), `KEY_TRACK`, `Node::key_tracks` (errors kept) and `Node::tracks`; GI points stay a mesh's

## 2. Playing

- [x] 2.1 `exa_anim::tracks`: `clock` (free, once, reverse, loop; zero-duration loop as free), `position_at` (the original's Hermite curve and end slopes, clamped outside the keys), `swapped` (once ↔ reverse)
- [x] 2.2 `TracksPlugin` in `AnimPlugin`: `NodeTracks`, `StartTracks` (the node and every node under it; skip playing tracks, swap the mode, log the start), `PlayingTracks` (virtual clock, translation from the curve, ended tracks and despawned nodes leave the list); F7 starts every track
- [x] 2.3 `exa_assets::ModelNode::tracks` from the scene node; `exa_world::spawn_model` inserts `NodeTracks` (`exa_world` depends on `exa_anim`); empty tracks in the other model-node constructors

## 3. Tool

- [x] 3.1 `exa-tool corpus tracks`: every track of every chunk tree, nested packs included, with file, node, parent, mode, duration and keys (s, cm)

## 4. Tests

- [x] 4.1 `key_track`: the shipped shape decodes in metres (z −5 cm at 1 s); mode words 0/1/2/3/7; truncated, short and overflowing payloads are errors, trailing bytes ignored
- [x] 4.2 Corpus (skipped without `EXANIMA_DIR`): every chunk tree of every pack and nested pack (more than 7000) holds exactly one track, `Objects/uwa01 brkswtch` `Box001` under `Plane001`, once, 3 s, keys (0, 0), (1, −5 cm), (2, −5 cm), (3, 0); no undecodable track; no collision body or cloth marker on a node kind other than a mesh; the existing body and cloth corpus tests unchanged
- [x] 4.3 `tracks`: the curve at 0.5/1.5/2.5 s is −2.1875/−5.625/−2.8125 cm, the keys on the keys, the ends clamped (NaN at the first key), one key, no keys, shuffled keys; the clock by mode (once 3.5 → 3 ended, reverse 0.5 → 2.5, 3.25 → 0 ended, loop 4.5 → 1.5, zero-duration loop free); the swap; an app: nothing plays unstarted, the first start reverses (0 at the start, −2.8125 cm at 0.5 s, −5.625 cm at 1.5 s), a start while playing changes nothing, it ends at rest after 3 s, the next start plays forward (−2.1875 cm at 0.5 s); a despawned node leaves the list
- [x] 4.4 `exa_world`: a placed model's child node with a track gets `NodeTracks` and moves on a start (−2.8125 cm 0.5 s in) while the placement stays

## 5. Verification

- [x] 5.1 `exa-tool corpus tracks` against the installation: 7653 chunk trees, 1 track, 0 undecodable
- [x] 5.2 In game, light runs through `gpu_run.sh --gpu 4000` (`--no-gi --no-supersample --size 960x540 --all-rooms`, camera beside the switch at −40.15, 1.93, 82.6, yaw 144°) of `level exanima03`: F7 logs "keyframe track 0 of Box001 started: Reverse, 3 s, at (−40.65, 1.93, 83.29)" and, 3.004 s later, its end at the rest position; the rest shot shows the brick standing out of the wall, a shot 2.2 s of game clock in (`--sim-speed 0.25`) shows it sunk in, and a shot after the end shows it back as at rest; no panic or validation error
- [x] 5.3 Remove probes and scratch files from the tracked tree (the sub-chunk probe and the light count were temporary and are gone)

## 6. Documentation

- [x] 6.1 `docs/formats/RFC.md` (Scene nodes: sub-chunks after every kind's body with their counts; Keyframe tracks), `docs/DEBUGGING.md` (F7, Keyframe tracks), `docs/PLAN.md` (Phase 1 table, Phase 2), `crates/exa_formats/README.md`, `crates/exa_anim/README.md`, `crates/exa_world/README.md`, `crates/exa_tool/README.md`, the launcher's key list
- [x] 6.2 `openspec validate --all --strict`
- [x] 6.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
