# Tasks

- [x] Trace the install, the three checks, the lightning raise, end and hit hook, the self-sharpening check and hit hook in REA, against the disassembly.
- [x] Trace the lit-blade byte: its writers (the moveset choice, lighting) and its reader (the two-handed grip callback).
- [x] Trace the socket control's drop, drag start and end, the card's commit and the give-back.
- [x] Read the four ability records from the installed `objdb.rdb` on the worker (skipped without `EXANIMA_DIR`).
- [x] `exa_game::item_effects`: rename the misread glow helpers to the self-sharpening rules; tests.
- [x] `exa_game::item_ability`: every character, types 1 and 2, the byte and the grip re-run, hit hooks, `--ability-item`; tests.
- [x] `exa_game::thaum::arcs`: the hum arcs' preset.
- [x] `exa_character`: `BurningGrip`, `HeldAbility`, the moveset choice's reset, the grip callback's read.
- [x] `exa_game::socket_drag`, `panel_live`, `item_card`: drop on a socket, drag out, give-back, commit; tests.
- [x] Crate READMEs (docs/formats and docs/PLAN.md unchanged: no format change; PLAN.md is the main session's).
- [x] GPU check in the one-window grid: the socket drag in and out, the mace's arcs and hits, the self-sharpening runs and hits, a duellist's lit blade.
- [x] fmt, clippy -D warnings and tests for the touched crates on the worker (`make check` equivalent).
