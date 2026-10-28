# Tasks

- [x] Check the residency (distance, sort, build pick, texture handoff, initial textures) against the assembly.
- [x] Trace the thin byte to the mesh set-up and the material loader's class, flags and opacity words.
- [x] Trace the top-slice flag to the environment record and the injection's instance count.
- [x] Add `GiResidency` with tests and wire it into the block builds; key the block cache by layout index.
- [x] Add `Material::is_thin_for_gi`, `ModelNode::gi_thin` and the `GiThinMesh` spawn with tests.
- [x] Carry `GI_TOP_SLICE` to the shader; add `drawn_slices` with a test.
- [x] Update `docs/RENDERING.md`, `docs/formats/RFC.md` and the crate READMEs.
- [x] fmt, clippy `-D warnings` and tests on the worker.
- [x] `openspec validate --all --strict`
