# Design

## Behaviour

The original keeps a character's possessions in its record: an out and a
stowed weapon set of two hands each, a worn list, and a carried list in
which an item in a hand stays listed with bit `0x100`. Every rule here acts
on that record the way the original's routines do; the body side (loading
held models, attaching them to the hand bones) is described by a plan and
left to the code that attaches models.

Item windows are retained-mode forms in the original. Their geometry,
callbacks and decisions are pure functions over plain data so that the GUI
forms, when they exist, only draw.

Item copies follow the original's arithmetic exactly: the mixed float and
double condition factor, ties to even, bytes that wrap before a cap, the
engine LCG (`×0x08088405 + 1`), its float draw (`state × 2⁻³²`), and its
fast logarithm and exponential polynomials for the random-table weights.

## Model

- `exa_formats::containers`: section lookup, `ModelFacts`, `Container`,
  `RandomTable`, `RowCondition`, the compact price codec.
- `exa_items::possession`: `Possessions` (a component), `hand_plan`,
  `hold_plan`, `displaced_value`, `takes_core`, `PlannedArms`.
- `exa_items::catalogue`: `ItemCatalogue` (a resource built from
  `objdb.rdb`), `SwapRequested`, `PendingHands`, the systems.
- `exa_items::inventory_window`, `arsenal`, `icon_cache`, `item_gui`.
- `exa_items::instance`, `palette`, `refresh`, `randomise`.
- `exa_game::item_container::Entry` and the panel helpers.

## Open questions

- What the original's body-state bits `0x10202` mean in Exarust's terms
  (the swap refusal passes 0). Answer with a capture of the state word.
- What sets the compact layout flag of the inventory and arsenal windows.
  Answer with REA on its writers.
- The armour variants' names (the six handlers are known by their numbers).
- The item-card text templates (`@nn` condition sections and gendered
  `{…}` words) and their pronoun table are not modelled.

## Docs

`docs/formats/OTHER.md` (Containers and random tables; Armour; the
appearance block's spreads and layer bytes; the race core-crystal flags),
`docs/PLAYER.md` (Weapon sets and what a character carries), `docs/UI.md`
(Item windows).
