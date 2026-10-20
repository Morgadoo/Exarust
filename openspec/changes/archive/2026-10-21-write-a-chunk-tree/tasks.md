# Tasks

- [x] Chunk framing (open, close, take back out) in `exa_formats::rfc_write`.
- [x] The walk, numbering, exported parent and data source.
- [x] Root chunks in the original's order: grid, materials and texture renames, scene nodes, props, local database, characters, environment.
- [x] The save game's placement writer.
- [x] Byte-exact and round-trip tests on synthetic scenes.
- [x] Corpus check: none needed; the writer reads no game data (the reader's corpus tests are unchanged).
- [x] `docs/formats/RFC.md` and the crate README. `docs/PLAN.md` is updated by the integrator.
- [x] `make check` for `exa_formats` (fmt, clippy -D warnings, tests) on the worker.
- [x] `openspec validate --all --strict`.
