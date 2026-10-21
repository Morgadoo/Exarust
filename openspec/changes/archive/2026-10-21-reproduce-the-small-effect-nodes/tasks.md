# Tasks

- [x] Verify every rule of the six components against the assembly and constants; correct the wheel's axes and labels, the shell's fade, the bubble's shrinking fade, the flame lag's axis and precision, the dissolve's jitter and push, the clock and the helpers' pairings.
- [x] Light the level portal marker a loaded model carries.
- [x] Check the installed packs for the portal emitter (one record, `_PortalPrt` in `uwportal01`); no test reads game data, nothing runs without `EXANIMA_DIR`.
- [x] Update `docs/formats/RFC.md` and the crate README; `docs/PLAN.md` is updated by the main session.
- [x] `make check` (remote: `cargo test -p exa_game`, `cargo clippy -p exa_game --all-targets -- -D warnings`).
