# Tasks

- [x] Review the texture word, sampler, colour space, first level and
  placeholder against the upload routine and its setters (REA pseudo-code
  and assembly); correct the merge mask, the data rule, anisotropy, the
  compressed-without-chain rule, the quality gate, the device limit, the
  bias order and the subsurface test.
- [x] Review the texture manager against its search, insert and name
  helpers; keep the first word, cut names at the dot, use the manager's
  order, count references on a duplicate insert.
- [x] Review the loader queue against its thread, promotion, priority load
  and push; correct promotion of decoded requests and the yield rule.
- [x] Review the content scheduler against its loop, pump, waits, fetches
  and unit init; serve the request rings in one pass, give streamed
  characters their ring of 128, leave requested characters for the fetch.
- [x] Corpus checks against the installation (skipped without
  `EXANIMA_DIR`): every shipped image converts and its chain flag agrees
  with its stored levels; the survey of unshipped material images.
- [x] Compare the per-image uploads of the previous loader and this one
  over the corpus (levels, sizes, formats, time).
- [x] Update `docs/formats/RFI.md`, `docs/RENDERING.md` and the
  `exa_assets` README. `docs/PLAN.md` is left to the integrator.
- [x] fmt, clippy `-D warnings` and tests for `exa_assets`, `exa_render` and
  `exa_formats` on the worker.
