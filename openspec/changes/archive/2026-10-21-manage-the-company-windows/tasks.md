# Tasks

- [x] Read the manage window, arsenal form, dialogs, status and records windows, report click, roster drag, sale hook, lesson pass, tournament start, bracket fill, unentered events, tourney forfeit, hub arrival and rematch routines; record them in `context/re/arena-manage.md`.
- [x] Add `exa_game::manage` (row labels, name ink, button gates, the arsenal form state, slider, points line, record texts, report click, company status) with tests.
- [x] Add `Staff::move_employee` and `Staff::lesson_round`, `Arsenal::give_up`, and `exa_game::tournament` (bracket, unentered events, event start, forfeit, hub arrival, rematch) with tests.
- [x] No corpus field changes (the routines read the executable's own tables; nothing to check against `EXANIMA_DIR`).
- [x] Update `docs/UI.md` and the `exa_game` README. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` are the integrator's.
- [x] `make check` (remote: `cargo test -p exa_game`, clippy `-D warnings`, `cargo fmt`).
