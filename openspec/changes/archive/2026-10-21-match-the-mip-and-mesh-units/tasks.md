# Tasks

- [x] Review the Kaiser kernel, Bessel and sinc helpers, both bank builders
  and the unit initialiser against the assembly; sum in f64, fix the sinc
  argument order, add the uniform bank and its choice by format word.
- [x] Review the four axis passes, their accumulate callbacks and the chain
  driver; read the wrap per axis from the image flags; gate the
  renormalisation on three- and four-channel words; use the engine's
  vector normaliser.
- [x] Review the window kernel builders, the resample driver and its
  dispatch; take the chain level for power-of-two ratios, pick the space by
  word and size, keep a zero integer ratio when magnifying.
- [x] Review the blits, canvas copy, sharpen kernel, kernel normaliser and
  bytes per pixel; blits take texel columns.
- [x] Review the corner-angle helper, the face-plane record, the normal
  builder, the tangent splitter, the bounds refit, the barycentric helper,
  the index-type rule and the regroup; correct the weight, the fallback
  faces, the tangent sign and zero tangent, the bounds floors and the
  barycentric projection.
- [x] Corpus checks (skipped without `EXANIMA_DIR`): every shipped image
  converts, with counts of generated chains by bank, clamp and
  renormalisation; every shipped mesh gets finite unit normals.
- [x] Update docs/formats/RFI.md, docs/formats/RFC.md, docs/RENDERING.md,
  docs/PERFORMANCE.md and the crate READMEs; docs/PLAN.md is updated by the
  coordinating session.
- [x] `cargo fmt`, clippy `-D warnings` and the tests of `exa_render`,
  `exa_assets` and `exa_formats` on the worker.
