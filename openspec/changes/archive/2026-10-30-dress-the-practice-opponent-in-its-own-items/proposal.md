# Proposal

## Why

The owner reported (with a screenshot) that the debug duellist of
`exarust --opponent play exanima01` has neither underwear nor trousers: it
shows a helm, gloves and boots over bare hips and legs. docs/PLAN.md Phase 5
(characters: the dressed character) and the practice ladder's opponent.

The cause is not the round-42 culling append. The practice template's worn
and held references carry the local bit: they name the items of the database
that follows the record in its `actors.rpk` entry. The viewer looked them up
by number in the game's object database, so opponent01's own trousers,
shirt and sandals (local items 2, 3 and 4) became the game's items 2 to 4:
gloves, a plate helm and boots. With no groin cover and no underwear marker,
the duellist was drawn bare from the waist to the boots. The bug dates from
the practice opponent's first landing, so round 41 shows it too.

Evidence: the character builder's item lookup in Exanima.exe (REA): a
reference with the local bit is found in the character's own object
database, one with the world bit in the world's, any other in the game's;
the corpus (`exa-tool characters actors`): opponent01 wears local 2
"6_L2_Bad Trouser…" (`Trouser_A`, covers the groin), local 3 "CltShirt LS A"
(`Shirty_A`) and local 4 "Sandals B" (`Sandals_B`). The generator's per-slot
culling append and its cull-bit choice over each garment's visible mask were
read again (REA): Exarust's order, masks and test match, and the shipped
braies, trousers and the outfits checked keep every hidden body slot under a
drawn garment.

## What Changes

- `exa_game::prelude`: the actor library keeps each template's own items;
  `template_item` routes a reference with the local bit to them, any other
  to the game's database.
- The viewer's debug duellist is dressed and armed from opponent01's own
  items, painted with their appearances, and weighed from its record.
- Tests pin opponent01's outfit and the shipped outfits' cull numbers
  (braies merged whole; the trousers' waist under a shirt).

## Out of scope

- The creator's background presets read templates for their records only;
  their item references are resolved where those records are dressed.
- The underwear rule itself (unchanged: it runs on records the game creates
  or re-dresses, not on a template at spawn); opponent01's trousers cover
  the groin, so it takes none.
- A retail capture of the practice opponent; the comparison is by corpus,
  REA and Exarust screenshots.
