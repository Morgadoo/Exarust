# Tasks

- [x] Keep each vertex's texture element and the triangle edge bits in decoded meshes.
- [x] Editable form, incidence, smoothing groups and render-form conversion (`exa_assets::mesh_edit`).
- [x] Vertex-cache order with the original's fast curves (`exa_assets::vertex_cache`).
- [x] Loop subdivision and the per-node pass with the bounds refit (`exa_assets::mesh_smooth`).
- [x] Polyhedra for the effect shells (`exa_assets::polyhedra`).
- [x] `ModelLoader::smoothed_model` and `exarust model … --msm`.
- [x] Corpus check `shipped_hair_smooths` (every `CharExt.rpk` mesh), skipped when `EXANIMA_DIR` is unset.
- [x] Update `docs/formats/RFC.md`, `docs/RENDERING.md`, `docs/DEBUGGING.md` and the crate README; `docs/PLAN.md` and the component map are the integrator's.
- [x] `make check` for the touched crates (remote tests and clippy).
