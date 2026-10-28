# Proposal

## Why

docs/PLAN.md, Phase 4 (combat and items), the item GUI paragraphs: the
description card's builders, its timer and toggle, the panel destroy's icon
release and the armour pass's encumbrance sum were tested rules without a
live host. The item description card was never drawn, a closed panel kept
every icon reference, and a body's encumbrance was only summed at spawn.

## What Changes

- The live description card (`exa_game::item_card`): a right press on a
  panel entry runs the panel's mouse decision (use class 0 and 1 wait on the
  200 ms timer, others build at once); the card is built from the record's
  card facts (`exa_items::catalogue::CardFacts`) through `exa_items::card`,
  holds an icon slot by the card's own acquire key, fades on the popup's
  squared curve, fades when the pointer leaves it and closes with the last
  item panel.
- Card facts read from the record: name and description through the
  original's string resolver (inherit, own-tier bit, shared library; an
  unresolved name gives no card), the compact value word expanded to copper
  before the quartering rule, and the tint/rank/socket bytes of the section
  whose page the card shows.
- Closing the inventory or a loot window releases every entry's icon
  reference; the last reference frees the slot and its atlas cell, and a
  freed slot is reused by the next acquire.
- A body whose possessions change has its encumbrance re-summed into both
  its physical and fighter stats; a new body keeps its spawn value.

Evidence: the card builder and its weapon, apparel and shield page
builders, the string resolver, the compact price decoder, the icon
manager's acquire and release, the panel destroy, and the armour pass
(Exanima.exe, read in assembly).

## Out of scope

- The generated quality and condition sentence: the phrase tables are data
  in Exanima.exe that Exarust has not extracted, so an apparel card, or a
  weapon card whose model name starts with `@`, shows its description
  without the sentence.
- The card's control layout (name, rows and value placement) is Exarust's;
  the card is placed from its least size, not its final height.
- Owner sex for the pronoun markers (male), the rank/points gates (off) and
  the value gate (on) are labelled guesses.
- The character view, the container popup, the socket and material
  controls, and refreshing an open card in place.
