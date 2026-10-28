# Tasks

- [x] 1. Read the phrase tables and variant words from the executable (`exa_formats::desc_phrases`) with a synthetic PE test
- [x] 2. Verify against the installed executable (`real_executable_has_every_table`, skipped without EXANIMA_DIR)
- [x] 3. Load the tables once, choose the sentence rule and rewrite title and description (`exa_items::desc_phrase`)
- [x] 4. Record the sentence source in the catalogue and wire it into the live card (`exa_game::item_card`)
- [x] 5. Build a whole sentence from the installed tables (`real_tables_build_a_sentence`)
- [x] 6. Update docs/formats/OTHER.md and the exa_formats, exa_items and exa_game READMEs (docs/PLAN.md is the integrator's)
- [x] 7. fmt, clippy -D warnings and tests for exa_formats, exa_items and exa_game
