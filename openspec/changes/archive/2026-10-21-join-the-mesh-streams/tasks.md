# Tasks

- [x] `Mesh::merge`: flag default 1, element-0 texture coordinates, GI points untouched; tests with hand-worked indices.
- [x] `Mesh::shadow_cull_front` replaces the cull-front material variant; the node copy and pipeline key stay as on main.
- [x] The colour stream stays off the GPU; `group_mesh` test asserts it.
- [x] `exa-tool corpus colours`.
- [x] `docs/formats/RFC.md`, `docs/RENDERING.md` and the crate READMEs.
- [x] Mip unit: power-of-two level search and kernel normaliser re-read against the assembly.
- [x] fmt, clippy -D warnings and tests for `exa_formats`, `exa_assets`, `exa_tool` on the worker.
- [x] `openspec validate --all --strict`.
