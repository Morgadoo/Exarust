# Tasks

- [x] Verify the first pass's rules against the assembly of the vegetation, terrain unit, importer and room manager
- [x] Drop the plants' vertex tint; compute the density in double precision
- [x] Fall back to `default.ftb` only for a missing palette; upgrade version 1 palettes field by field
- [x] Replace the fading flag on meshes only; keep the database-only read to records and strings
- [x] Spill light from the cell centre; read portal sizes as half widths and tops
- [x] Nest placement transform records, match names exactly, rebuild models for position lists
- [x] Corpus checks with EXANIMA_DIR (skipped without it): shipped placement records resolve and match; navigation pins
- [x] Update docs/formats/TERRAIN.md, docs/formats/RFC.md, docs/RENDERING.md, docs/NAVIGATION.md and the crate READMEs (docs/PLAN.md is updated by the main session)
- [x] `cargo fmt --all`, clippy -D warnings and the crate tests on the worker
