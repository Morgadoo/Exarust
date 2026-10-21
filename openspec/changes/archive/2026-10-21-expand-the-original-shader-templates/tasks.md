# Tasks

- [x] Decode `.flb` libraries with total errors and the engine's NUL rules
- [x] Expand templates with the builder's condition, chain, loop, expression, printing, include and interface rules
- [x] Build the material and shader words, program choice, sample splits and warm-up keys as the factory and builder do
- [x] Pack the `Global`, `Material` and light uniform values as the writers do
- [x] Add `exa-tool shader` with `--warmup`, `--list`, `--all` and `--dump`
- [x] Correct the WESL surface program against the decoded templates; revert what is not their maths
- [x] Verify against the installation: dump and compare all 54 entries, expand every entry, survey the four material libraries (skipped without `EXANIMA_DIR`)
- [x] Update docs/formats/FLB.md, docs/formats/README.md, docs/RENDERING.md and the crate READMEs (docs/PLAN.md is updated by the main session)
- [x] Run tests and clippy (`-D warnings`) for exa_render, exa_formats, exa_tool and exa_assets
