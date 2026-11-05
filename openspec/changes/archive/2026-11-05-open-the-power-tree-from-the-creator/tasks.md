## 1. Creator

- [x] 1.1 `apply_creation_defaults` and the unknown background's entry record
- [x] 1.2 The final button reads the record's powers, keeps its label for one with powers, and opens the tree (page state 2, form hidden)
- [x] 1.3 BACK and BEGIN answers (`tree_answered`, `tree_begin_kind`)

## 2. Tree

- [x] 2.1 `CreatorPowerTree`: open in creation mode on the creator's record; prompt buttons and Enter/Escape answer; the game's record restored
- [x] 2.2 `PowerRecord::of_character` and `write_into`
- [x] 2.3 The tree's text atlas follows the interface unit; the skin text program reads the atlas as the GUI text draw does
- [x] 2.4 Remove the invented `TreeScreen::commit`

## 3. Story start

- [x] 3.1 The power word into the record's power word, the value into the potential

## 4. Checks

- [x] 4.1 Unit and app tests (creator flow, tree request and answers, record round trip, story writes); the prompt's title drawn with the installed packs after a unit change (EXANIMA_DIR, skips without it)
- [x] 4.2 GPU check in the one-window grid: `--open-creator powers`
- [x] 4.3 fmt, clippy -D warnings, tests for exa_game and exarust (on the worker)
