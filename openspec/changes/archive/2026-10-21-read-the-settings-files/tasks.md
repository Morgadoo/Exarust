# Tasks

- [x] Read the settings cluster, `RF_StoreSettings` and `RF_DataIO` with REA; settle the search base, the splice size and the handlers from the instructions.
- [x] `exa_formats`: `settings_ini`, `settings_store`, `data_stream`, `file_ref`, with tests pinned to the original's numbers.
- [x] `exarust`: read the files at launch, apply the window, vsync, supersample, readout and GUI size, write the first run, use the store's bindings, save the store on exit; `--no-settings`.
- [x] Docs: `docs/formats/SETTINGS.md` and its link in `docs/formats/README.md`, `docs/UI.md`, `docs/PLAYER.md`, the `exa_formats` and `exarust` READMEs. `docs/PLAN.md` is the integrator's.
- [x] No corpus check applies: user files are not shipped, and loose files stay skipped when `EXANIMA_DIR` is unset.
- [x] `make check` for the touched crates (tests and clippy on the worker).
