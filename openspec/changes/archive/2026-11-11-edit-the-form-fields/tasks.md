## 1. Changes

- [x] 1.1 `exa_ui::field_edit` (text field and combo key handlers, caret press and drag, origin, focus selection, swatch and emblem drop) with tests
- [x] 1.2 Focus follows the pointer in the hub's forms; keys through the handlers with the system clipboard; `--press` key names
- [x] 1.3 Field callbacks: emblem name on Enter, picture numbers, merchant re-store, loadout list step
- [x] 1.4 Swatch colour drag with the drag threshold and its draw; drop on the emblem swatches
- [x] 1.5 Text field front draw through the edit unit (selection, caret); combo front draw with its edit bytes

## 2. Checks

- [x] 2.1 Tests `the_emblem_name_edits_at_the_caret_and_klean_waits_for_enter`, `a_picture_number_takes_digits_and_selects_on_enter`, `a_merchant_combo_edits_its_text_and_enter_restores_its_choice`, `a_swatch_drag_drops_its_colour_on_another_swatch`
- [x] 2.2 GPU grids (company and merchant hub windows, before and after)
- [x] 2.3 Duel and exanima01 lockstep traces equal the base's
- [x] 2.4 exa_ui and exa_game READMEs, docs/UI.md; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.5 fmt, clippy -D warnings, tests on the worker
