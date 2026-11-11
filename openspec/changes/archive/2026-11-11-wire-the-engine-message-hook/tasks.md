## 1. Changes

- [x] 1.1 `exa_game::engine_log`: hook, buffer and whole-file rewrite (not level 5, not while held), status history, overlay push and expiry; tests `every_message_goes_to_the_buffer_with_a_line_break`, `the_file_is_rewritten_whole_except_for_level_five_and_while_held`, `levels_present_as_the_original_does`
- [x] 1.2 The overlay drawn after the interface through `GuiFrames` (soft box style 3 alpha 0x80, main font, eighth label ink), nothing while empty
- [x] 1.3 Script reports, display-mode refusal and package failure routed through the hook
- [x] 1.4 `exarust`: log file `<user folder>/<project>.log`, the hook's message boxes
- [x] 1.5 `EXA_LOG_PROBE` / `EXA_LOG_PROBE_FRAME`; test `the_probe_reads_level_title_and_text`

## 2. Checks

- [x] 2.1 Grid, arena1 with the probe: the overlay shows "Script : Invalid role set for the witness event." wrapped in the top-left corner and is gone later; the log file holds the line with CR LF
- [x] 2.2 Lockstep digests of arena1 and exanima01 against round 182's build
- [x] 2.3 fmt, clippy -D warnings, exa_game, exa_ui and exarust tests on the worker
- [x] 2.4 docs/PERFORMANCE.md, docs/DEBUGGING.md, crate READMEs; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
