## 1. Changes

- [x] 1.1 `Step::ExitNudge` in the story continue, kept with the placement across the load; `level_change::nudge_out_of_exits`; the viewer's placement applies it before the floor cast; tests `a_continued_player_in_an_exit_is_nudged_out`, the continue test
- [x] 1.2 `slots::offers_portal_reset`, `SlotList::reset_to_portal`; test `the_reset_keeps_a_slot_with_a_checkpoint`
- [x] 1.3 The saves' screen's reset box (open and select, hit, Yes, No, cancel, paint); test `a_legacy_save_past_the_portal_asks_to_reset`
- [x] 1.4 `exa_ui::dialogs::RESET_RECORDS`; test `the_reset_dialog_asks_and_answers_yes_or_no`
- [x] 1.5 `--portal-reset yes|no`

## 2. Checks

- [x] 2.1 Grid: a scratch story save made pre-0.9.5 (flags byte cleared, level section renamed exanima06) raises the box when the saves' screen opens; Yes deletes the .rsg and drops the slot (no checkpoint)
- [x] 2.2 Grid: a continued save runs the nudge over the level's exits (none held the player in the scene tried)
- [x] 2.3 Lockstep exanima01 traces identical to round 200 over 968 common ticks; determinism grid identical over common rows
- [x] 2.4 fmt, clippy -D warnings, workspace tests, openspec validate
