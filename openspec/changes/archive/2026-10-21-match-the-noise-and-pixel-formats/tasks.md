# Tasks

- [x] Reproduce the noise unit in `exa_render::image_noise`, with tests
  pinned to values worked by a separate model of the rules.
- [x] Build the terrain noise maps from it and fold map A once.
- [x] Reproduce the pixel formats and converters in `exa_formats::pixel`,
  with tests on worked bytes; run the decoder through them.
- [x] Filter generated mips in the space the format word picks.
- [x] Update `docs/formats/RFI.md`, `docs/formats/TERRAIN.md`,
  `docs/RENDERING.md` and the crate READMEs. `docs/PLAN.md` and the
  component map are left to the integrator.
- [x] No corpus fields change; the corpus decode checks still skip when
  `EXANIMA_DIR` is unset.
- [x] `make check` for the touched crates (tests and clippy on the worker).
