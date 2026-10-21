# Tasks

- [x] Correct the colour roll, tables, deforms and placement (`exa_formats::factory`, `procedural`, `palette`) and share the roll with the item palette.
- [x] Use the stat pass's own curves and precision (`exa_formats::weapon_stats`).
- [x] Paint factory regions as the original's job and correct the material programs (`exa_assets::paint`, `model`, `weapon`).
- [x] Corpus checks, skipped when `EXANIMA_DIR` is unset: every shipped factory parses, rolls and assembles with a mesh and a painted material per part; stored choices match a fresh roll.
- [x] Update `docs/formats/OTHER.md` and the crate READMEs; `docs/PLAN.md` and the component map are the integrator's.
- [x] `make check` for the touched crates (remote tests and clippy).
