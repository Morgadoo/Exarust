# Tasks

- [x] Build each 6 m block's mesh from the shared 30 × 30 patch; model the
      LOD shift, the row count and the 16-bit indices.
- [x] Build tiles on the async compute pool and start the next tile ahead.
- [x] Add `TerrainGround` with the engine floor and the slot rule.
- [x] Add the streaming rules with the original's compares.
- [x] Apply the lit program's frame to the normal map and the parallax;
      mask the specular after `MatDif`; keep the mirror's singular point at
      0.
- [x] Real-data checks on the worker (skipped without EXANIMA_DIR): the
      shipped tile centres, tiling factors, palette floats, every material
      kept, the tile worker on exanima01, the stepped level build.
- [x] Update `docs/formats/TERRAIN.md`, `docs/RENDERING.md` and the crate
      READMEs (docs/PLAN.md is updated by the session that merges).
- [x] `cargo fmt`, clippy with `-D warnings` and the tests of `exa_world`
      and `exa_render` on the worker.
- [x] `openspec validate --all --strict`
