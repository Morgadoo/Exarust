## 1. New game slot

- [x] 1.1 Read the session begin, the empty-slot creator and its callers in the assembly
- [x] 1.2 `Step::NewSlot` at the begin of a story or a career; the slot in the mode's manager
- [x] 1.3 Tests over a scratch save folder: the slot is made, named and current; the first save lands in it

## 2. Arena saves

- [x] 2.1 Read the save writer, the board Begin, the rematch callback, the quit hook and the forfeit save
- [x] 2.2 Save steps on the arena manager; the forfeit's player-data save
- [x] 2.3 The Begin and rematch saves before the launch; the quit's autosave in `Last`
- [x] 2.4 Tests: arena save and forfeit save over a scratch folder, the quit, the quit's forfeit, the rematch

## 3. Golem alarm and journal

- [x] 3.1 Read the settings chunk's writer, reader and the alarm trigger
- [x] 3.2 The live alarm into the story section; its restore from a save's section
- [x] 3.3 The live journal into the save; the loaded journal into the in-game journal
- [x] 3.4 Tests: the chunk round trip, the restore, the journal round trip
