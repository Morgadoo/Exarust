# Tasks

- [x] Verify every rule of the 77 routines against the assembly and correct the technique selector, the compact skill screen, the loadout form, the divination reading, the emblem editor and the destination screen.
- [x] Wire the destination screen to `OpenPortalScreen`, the bubble and the teleporter's glow; the technique selector and the loadout editor to the roster member.
- [x] Real-data check: none applies (the forms read no game file; the teleporter records were checked against the installed `objdb.rdb` by the devices change), so no `EXANIMA_DIR` test is added.
- [x] Update `docs/UI.md` and the crate READMEs; `docs/formats/` is unchanged; `docs/PLAN.md` is updated by the main session.
- [x] `make check` (remote: `cargo test -p exa_ui -p exa_game`, `cargo clippy -p exa_ui -p exa_game --all-targets -- -D warnings`).
