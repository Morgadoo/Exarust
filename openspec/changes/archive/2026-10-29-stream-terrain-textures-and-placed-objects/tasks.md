# Tasks

- [x] Check the terrain path, the fast-load branch, the level wait's message
  and room-mode test against the original's instructions.
- [x] Queue and wait for the terrain's palette textures; finish them on the
  spot for a one-call load.
- [x] Add the fast-load flag to the texture manager.
- [x] Drive the placed objects through the content stream in `LevelLoad`;
  report the missing count and the player's position.
- [x] Real-data checks: a terrain level loaded in one call with streaming
  on, and the stepped load equal to the one-call load (EXANIMA_DIR).
- [x] Update the exa_assets and exa_world READMEs (no docs/formats change).
- [x] fmt, clippy -D warnings and tests for exa_assets, exa_rpk, exa_render
  and exa_world.
