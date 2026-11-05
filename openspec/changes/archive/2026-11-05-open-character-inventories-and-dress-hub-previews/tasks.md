# Tasks

- [x] Trace the inventory call, its finder, bind and close, and its callers (key, interaction hook, `ShowInventory`) in REA against the assembly.
- [x] Trace the character view's accept, drop and release, the trade window's constructor, opener, layout, drop-back and give-back, and the loadout form's preview, kit list and box callbacks.
- [x] Corpus: scan `charroles.rdb` of the installed build for interaction-condition sections (two: Golem Boss, Golem Servant).
- [x] Bind the inventory window to a character; ShowInventory and the interaction open it; consumers read the bound body; tests.
- [x] Loadout preview: dress from the kit list, pick and take off into the kit list or back to the arsenal; tests.
- [x] Trade preview: the copy, the drop, the drop-back and give-back, the take-off; tests.
- [x] `--character-use` debug tool.
- [x] Crate README (docs/PLAN.md is the owner's; no docs/formats page changes).
- [x] Real installation: GPU runs of `play exanima01 --searcher 0x2b --character-use 43@40+` and `--encounter-script "Actor[0].ShowInventory;"`, `play arenahub --open-hub-window loadout` and `shop` with drags onto and off the preview.
- [x] fmt, clippy -D warnings and tests for the touched crates on the worker; play build of `exarust`.
