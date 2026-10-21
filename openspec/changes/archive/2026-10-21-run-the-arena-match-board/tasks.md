# Tasks

- [x] Model the selected match record, the six standard matches and the
      coin scaling, with tests for the fee, prize, tier clamp and greyed rows.
- [x] Model the lineup rebuild and the four move paths, with tests for the
      leader rule, bench, swap, marked swap, Captain checks and lineup order.
- [x] Model the daily event, winner and season tables, the tournament status,
      the weapon check, the message, the label and controls, the show, close
      and tab switch, layout, paint order, hit test and clicks.
- [x] Fill a board from a career and the daily board in `exa_game`.
- [x] Update `docs/UI.md` and the `exa_ui` and `exa_game` READMEs.
- [x] No corpus files are read; loose-file checks stay skipped when
      `EXANIMA_DIR` is unset.
- [x] `docs/PLAN.md` and `docs/COMPONENT-MAP.md` are updated by the main
      session from this change's plan note.
- [x] `make check` (fmt, clippy -D warnings and tests for `exa_ui` and
      `exa_game`, run on the build worker).
