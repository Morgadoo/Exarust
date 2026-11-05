# Tasks

- [x] Trace in REA, against the assembly:
  - the scripted-mode table's lookups (mode switch, session start and leave, back to the title, creator cancel, end menu);
  - its finaliser;
  - the writers of its count, the versus flag and the two lobby hooks;
  - the main menu's page builder and click;
  - the creator's final button.
- [x] Model the table in the flow, with first-match lookups and the original's hook slots; drop the separate end-menu handler resource.
- [x] Correct the enter, begin and leave lookups. Put the creator and class-select cancels behind the transition with the original's hooks.
- [x] VERSUS switches the mode word; READY with no lobby form keeps the creator.
- [x] `--versus` debug flag; `--press` letters and digits also type.
- [x] Unit tests: unregistered and registered words, the bit test, the cancel slot, the hook order.
- [x] Crate READMEs. No docs/formats page changes: no data format is involved. docs/PLAN.md is the owner's, so no PLAN edit.
- [x] Real installation: the menu → VERSUS → creator → READY → cancel → title round trip, practice and story round trips, on the GPU through the grid.
- [x] fmt, clippy -D warnings and tests for exa_game and exarust on the worker; play build of `exarust`.
