# Proposal

## Why

Wiring backlog P5 (object use effects: the readable parchment and the
divination card) and the rest of P6 (copied item bodies). The parchment
popup's draw and hit were ported with no opener; a divination deck's use
sent `OpenDivination` (31 October) but the reading drew nothing, because its
card sheet was never loaded and its paint ran before the screens' draw
cleared the GUI queue; the shop's copies were stand-in ids with no item
body, so a bought item read its template's condition.

Phase: Phase 4/7 item and arena milestones of docs/PLAN.md (no PLAN.md edit
in this change).

Evidence (REA, Ghidra on `Exanima.exe`): the item GUI's card builder (its
map branch before the container and card branches), the parchment opener,
the parchment's constructor, destroy, mouse, hit and draw slots and the
compressed-texel sampler its hit reads; the base form's show; the trade
unit's stock update and its two copy helpers (apparel, and the grade
routine at level 1); the object database's instance dispatcher and guarded
removal; the apparel palette generator; the apparel and weapon template
refreshes; the shared random unit. Data: every map section in
`Resource.rpk` (the base tier's "Map" is record 1076; each story level has
its own, `exanima01`'s is local 9).

The backlog's reading of kind 7 as "open the parchment" is not the
original's: kind 7 hands the header name to the level loader (already
wired on 31 October), and the parchment is the card builder's answer for a
map.

## What Changes

- `exa_formats::objdb`: `map_image` (the map section's image name);
  `ObjectDb::set_payload`.
- `exa_items::catalogue`: `CardFacts::map_image`.
- `exa_game::parchment` (new): the map's parchment, opened by the card
  builder for a map in place of a card (`item_card::show`), sized and
  placed by the original's rules, fading in on the card's 256 ms, hit
  through its texture's alpha and faded out by a right press, drawn as one
  white textured quad.
- `exa_game::div_card_screen`: the reading loads its card sheet when it
  opens and paints after the screens.
- `exa_game::use_cursor`: the panel double click's dispatch as one routine;
  `--double-click ID@FRAME` runs it on a pack entry.
- `exa_game::shop_stock`: `LiveShopWorld`, the stock update's world over the
  live object database: an apparel copy is instanced, takes its quality from
  the shared seed, rolls the palette for its wealth and is refreshed from
  its template; a factory weapon with the instance bit is graded at level 1
  and refreshed (template bytes, roll, factory stats, condition pass); a
  dropped copy is removed from its tier. `ItemPalette`, `ShopFactories`.
- `exa_game::arena_hub_live`: the hub shop and the night's restock run over
  `LiveShopWorld`; a copy's shelf price is its own value word; each copy is
  logged with its condition; `--open-hub-window shop` opens the shop as the
  hub's click does; `--shop-buy ROW` buys a stock row through the window's
  clicks and opens the copy's card.

## Out of scope

- The map parchment's hit reads the texture's own pixel; the original reads
  the compressed block (approximation).
- Shop copies live in the hub's local tier; carrying them across a level
  change or an arena save is the arena save's promotion (S2 follow-up).
- The trade window still draws plain names, not the description phrase.
