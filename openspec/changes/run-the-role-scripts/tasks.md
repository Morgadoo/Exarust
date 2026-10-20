# Tasks

- [x] Read role records, entries and dialogue graphs (`exa_formats::roles`), with synthetic tests and errors for truncation and deep nesting.
- [x] Verify against the installed `charroles.rdb` and `narrator.rcd` (49 roles, 5 narrator nodes); skipped when `EXANIMA_DIR` is unset.
- [x] Run scripts as the original walks them (`exa_script::interp`, `builtins`), with the shipped script shapes as tests.
- [x] Roles, relations and topic memory (`cast`, `topics`).
- [x] Speech bubbles and the conversation engine (`bubbles`, `dialogue`): search, start, evaluate, step, end, events, topics, panel, narrator, witnessed and broadcast events.
- [x] Scripted objects and the crystal drain (`objects`).
- [x] `ScriptPlugin`: load the data, step the engine on the virtual clock, messages in and out.
- [x] Corpus checks of every shipped script and of Derrin's topics (skipped without the game).
- [x] Update `docs/formats/OTHER.md`, `docs/UI.md` and the crate README. `docs/PLAN.md` is updated by the integrator.
- [x] `make check` equivalent on the worker: tests and clippy `-D warnings` for `exa_formats`, `exa_script`, `exa_game`.
