# Tasks

- [x] Trace the object use, its script run, the operative script accessor, `WitnessRole` and its wrapper in REA against the assembly.
- [x] Trace the roles unit's `StartDialogue`, the encounter's static context, the dialogue AI entry and its callback.
- [x] Trace the controls' interaction hook, its condition test and the window it opens.
- [x] Corpus: grep `objdb.rdb` (one use script, the Pedestal Hand's) and `charroles.rdb` (one `StartDialogue`, a dialogue node action).
- [x] Operative use script in `exa_formats::objdb` and `ObjectSections`; tests.
- [x] Device use runs the script, `WitnessRole` from the operative; tests.
- [x] Encounter context in the engine; `StartDialogue` between that pair; `exa_ai` model removed; tests.
- [x] Character interaction condition and markers; tests.
- [x] `--encounter-script` debug tool.
- [x] Crate READMEs and `docs/formats/OTHER.md` (docs/PLAN.md is the owner's).
- [x] Real installation: GPU runs of `play exanima67 --object-click 300#587` and `play exanima01 --story-session --encounter-script "StartDialogue(Greeting);"`.
- [x] fmt, clippy -D warnings and tests for the touched crates on the worker; play build of `exarust`.
