# Tasks

- [x] Trace the factory weapon builder, its body and sound set, and its five callers' "no body" argument with REA. Confirm that the hand attach does not branch on a factory weapon.
- [x] Read the striking part's classification of a pair hit event (kinds 1–3, blade side, pending and overwrite rules) from context/re/body-pairs.md §11.3 and master-combat.md §5.
- [x] `exa_character::item_bodies`: attach `assembled_item_body` to factory weapons in the main hand, the off hand and the GUI drop, with the factory's hit set. Extend the assembled-body test.
- [x] `exa_combat::block::strike_held`: kind 1 wounds from a held weapon body's hit event on a limb. `detect_hits`: no sweep for held weapon bodies.
- [x] Corpus check: `@GlmHalbrd A` and `@UBM Axe1H A1` fitted from the installation, logged at the equip in the GPU runs.
- [x] GPU runs: `exanima06` golems against the player (`--at 10.5,0,65 --press W@400-560 --undying`) before and after; `exanima01 --opponent` sword duel before and after.
- [x] docs/PLAN.md and docs/COMPONENT-MAP.md are left to the orchestrator. No docs/formats/ page changes. Crate README updated.
- [x] fmt, clippy -D warnings and tests for the touched crates on the worker (`make check` scope).
