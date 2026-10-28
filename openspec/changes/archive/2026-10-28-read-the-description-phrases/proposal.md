# Proposal

## Why

docs/PLAN.md, Phase 4 (combat and items), the live item card milestone: its
remaining step was "extract the description phrase tables from the
executable for the sentence builders". The weapon and apparel sentence
rules were tested functions over caller-supplied rows; the running card
showed the bare description, without the generated quality and condition
sentence and without the armour-variant word in the title.

## What Changes

- `exa_formats::desc_phrases` reads the five phrase tables (11 flawless,
  10 perfect, 8 high-quality, 6 great and 15 made rows, each a text and an
  inclusive 0–100 mark range) and the eight armour-variant words from the
  installed `Exanima.exe`, through a total PE32+ reader. Tables are found by
  the pointer to their first phrase; no address and no table text is
  shipped.
- `exa_items::desc_phrase` loads them once from the game folder
  (`PhraseBook`), chooses the card's sentence rule as the card builder does
  and rewrites the title and description (`describe`).
- The catalogue keeps each record's sentence source; `exa_game::item_card`
  rewrites the description before its condition blocks and pronoun markers
  and shows the apparel title with its variant word.
- Fix: the apparel sentence reads 15 made rows, as its loop does; the
  inventory-pane spec still said 14.

## Evidence

REA on `Exanima.exe` 0.9.5: the card builder's sentence branch (its register
use decides the shield fall-back), both sentence builders and the phrase
pick, checked against the instructions (`DIVSD` by the double 2.55, then
`CVTSD2SI` under round-to-nearest-even; the made-row loop bound); a parse
of the installed executable.

## Out of scope

The sample plate-armour sentence (no caller in the original) stays a tested
function over caller rows. The engine's single shared generator is not
modelled: the card keeps its own roll. Localisation of the tables.
