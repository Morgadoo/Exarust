## 1. Board data

- [x] 1.1 Read the lineup rebuild, the two drops, the event value accessor and the weapon mark in the assembly
- [x] 1.2 `board_with`: entries, winners, event values and the order list
- [x] 1.3 Tests: the board reads the career's rows; the write-back and the removal

## 2. Board tables

- [x] 2.1 Read the board constructor's table callbacks (double presses, drag-over check, drops)
- [x] 2.2 `board_table_input` and its system before the hub's clicks; `--board-table`
- [x] 2.3 Headless app tests: a double press, a tournament drag and its way back, a drop on its own table

## 3. Week winners and the report

- [x] 3.1 Read the week verdict's slot write and the report's name lookup
- [x] 3.2 `week_winners` in the verdict, the save and the load; the report's hire list and names
- [x] 3.3 Tests: the verdict, the save round trip, the report lines

## 4. Session ends

- [x] 4.1 Read the end-session dispatch's arena item, the arena continue and the session leave
- [x] 4.2 The end menu's CONTINUE sends the arena's continue; the flow's arena leave step
- [x] 4.3 Headless app tests: the menu's CONTINUE starts the next round; the leave makes the hub busy

## 5. Checks

- [x] 5.1 Crate READMEs
- [x] 5.2 fmt, clippy -D warnings, workspace tests, openspec validate, play build
