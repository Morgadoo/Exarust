# Design

Rules: [`docs/formats/SAVES.md`](../../../docs/formats/SAVES.md), Loading a
level, Level transitions, Practice and Version conversion; the crate
READMEs.

## Decisions

- The record repair takes a decoded `CharacterRecord`; the raw
  `ArenaLists` form keeps the same order for byte lists.
- The scene loads, import, transition and kit passes are step lists and
  small results: the engine objects they drive are other units', so the
  rules are pinned as orders, branch outcomes and constants.
- A terrain entry found and read counts as loaded even when Exarust's
  parser refuses it, because the original's arena hook only hears of a
  missing file.
- Fields whose meaning is not traced (the environment word the named load
  clears, the grid word the menu load clears, the GUI helper both the
  transition and the combatant scene call) are named by where they live
  and labelled guesses.

## Not wired

Listed in the proposal's out-of-scope paragraph.
