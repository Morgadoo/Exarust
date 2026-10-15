## 1. Object database

- [x] 1.1 `exa_formats::objdb`: `Item.worn_model` from the body's first name, `Item.model` re-documented as the ground model
- [x] 1.2 `Item.occupancy` (body `+0x10`, 20 bones, zeros for a short body) and `Item::covers_groin()`
- [x] 1.3 Unit tests on synthetic bytes: worn model named by the body, item without a ground model, groin cover only on the innermost layer, short body

## 2. The rule

- [x] 2.1 `exa_character`: `UNDERWEAR_ITEM`, `HUMAN`, `needs_underwear`, the `WearsUnderwear` marker
- [x] 2.2 The underwear items from the object database, `put_on_underwear` and the `keep_underwear` system before dressing
- [x] 2.3 Item information keyed by worn model for apparel, by ground model for weapons
- [x] 2.4 `PlayerBody` requires `WearsUnderwear`; the character view marks a human body
- [x] 2.5 Unit tests: the rule, putting the underwear on, item information, only marked undressed outfits, the player body

## 3. Placed characters and tools

- [x] 3.1 `exa_world`: placed characters wear their items by worn model, with no rule applied; unit test on a synthetic scene
- [x] 3.2 `exa-tool paint` chooses its item by worn model
- [x] 3.3 `exa-tool corpus levels` reports placed humans, groin cover and worn items with a `Characters.rpk` model

## 4. Verification

- [x] 4.1 Corpus (skipped when `EXANIMA_DIR` is unset): `cargo exa corpus levels` reports 103 of 103 placed humans covered and 481 of 481 worn items with a `Characters.rpk` model (83 and 461 by ground model); `cargo exa paint braies_a braies__aa` names "Braies_A" and paints pale grey
- [x] 4.2 Screenshots: `exarust character`, `exarust character --outfit trousers_c`, `exarust play exanima01`, and Sticky next to a player in `--outfit trouser_a,shirt_b`, compared with the original's braies: the shape matches, the colour renders darker (design, Risks)
- [x] 4.3 Remove probes and dumps from the tracked tree

## 5. Documentation

- [x] 5.1 `docs/formats/OTHER.md`: section table (ground and worn model), record flags, name section, occupancy table, worn-model count, new subsection Underwear
- [x] 5.2 `docs/PLAN.md` (Phase 3 done line, Phase 1 objdb row), `docs/PLAYER.md` (start outfit), `crates/exa_character/README.md`, `crates/exa_tool/README.md`
- [x] 5.3 `openspec validate --all --strict`
- [x] 5.4 `make check`
