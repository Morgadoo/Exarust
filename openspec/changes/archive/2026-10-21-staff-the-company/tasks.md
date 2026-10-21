# Tasks

- [x] Read the hire, recruitment, renumber, kit, remove and dismiss routines and the manage window's report; record them in `context/re/arena-staff.md`.
- [x] Add `exa_game::staff` (classes, ids, fighters, hire, recruit, remove, dismiss, the order list, entry and log renumbering) with tests.
- [x] Add `exa_game::report` (day title, match phrase, log lines, ready-to-train, ranks, hire-list lines, tournament countdown) with tests.
- [x] No corpus field changes (the routines read the executable's own tables; skipped without `EXANIMA_DIR`).
- [x] Update `docs/formats/OTHER.md` and the `exa_game` README. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` are the integrator's.
- [x] `make check` (remote: `cargo test -p exa_game`, clippy `-D warnings`, `cargo fmt`).
