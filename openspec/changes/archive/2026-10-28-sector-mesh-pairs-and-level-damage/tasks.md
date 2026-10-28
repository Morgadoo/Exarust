# Tasks

- [x] Rebase the lane onto main.
- [x] Check the mesh query, its pre-tests and sweep choice against the contact generation's assembly; drop the invented static-only gate.
- [x] Check the pair friction gate's compare (parity branch) and restore the under-epsilon skip.
- [x] Check the level push gate's NaN and infinity paths.
- [x] Pre-test the sphere fallback against its own spheres' bound.
- [x] Check the level impact consumer against the motile part's impact routine; drop the limb impact message, add the overhead drop.
- [x] Update `crates/exa_physics/README.md`, `crates/exa_combat/README.md`, `docs/formats/RSQ.md`.
- [x] Clippy `-D warnings` for `exa_physics`, `exa_game`, `exa_combat` and the full workspace tests, remotely.
- [x] `openspec validate --all --strict`
