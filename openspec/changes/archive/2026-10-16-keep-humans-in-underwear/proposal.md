## Why

The player and the character view's body wear nothing at all: Exarust
spawns them with an empty outfit, and the bare human pelvis draws as a black
patch. The original never shows a human that way. Its body is nude, and the
engine keeps every human character record it creates or re-dresses in
underwear: when nothing worn covers the groin, the object database's braies
("Braies_A", item 8, pale grey linen) are added. A new game without a
background kit starts in braies only.

Placed characters lose clothes for a second reason. Exarust takes an
apparel item's model from the item's ground-model section, but the original
dresses a character in the model the item's body names. Item 8 has no
ground model, so the four braies wearers in the story levels (Sticky, two
dead men in exanima01, Fergus in exanima02) are dropped to their black
pelvis. Sixteen wearers of quilted and padded trousers ask `Characters.rpk`
for `Trouser_At`, which only exists in `Objects.rpk`, and walk about with no
trousers.

Phase 3 (Bodies), milestone M3 "standing man", which calls for a fully
dressed character walking a level.

## What Changes

- The object-database parser reads an item's worn model (the first name of
  its body) and its per-bone occupancy table, and tells whether an item
  covers the groin on the innermost layer.
- Apparel is dressed, painted and looked up by its worn model; weapons stay
  keyed by the model they show. `braies_a` is now painted with item 8's own
  appearance (rough cloth, pale grey) rather than the unused silk "Braies_P".
- The original's underwear rule, for humans Exarust makes at run time: the
  player in play mode and the character view's human body. The underwear
  goes on beneath the given outfit when nothing in it covers the groin.
- Placed characters wear exactly their stored items, each by its worn model;
  no rule is applied to them, as the original applies none at spawn.
- `exa-tool paint` chooses its item by worn model; `exa-tool corpus levels`
  reports the placed humans, how many are covered at the groin, and how
  many of their worn items name a `Characters.rpk` model.

Evidence:
- reverse engineering of `Exanima.exe` 0.9.5 through REA (the apparel model
  loader, the "ensure underwear" routine and its callers, the record
  defaults, the creator's background presets and kit tables, the equip
  conflict pass and the displaced-item handlers);
- a corpus census of `objdb.rdb`, every level's own object database and
  `actors.rpk` (1617 apparel records whose two model names agree, 3 + 39
  that differ; 103 placed humans, all covered by their stored items);
- a retail save whose fresh story player, background 0, wears item 8 only;
- reference screenshots of the original's character creation and of the
  player in braies.

Out of scope:
- inventory and equipping: the rule on unequip, braies vanishing when
  displaced and never being carried, wear conflicts between garments given
  on the command line;
- creator kits, story backgrounds and saves;
- the arena's NPC and hireling outfit generators;
- fit variants and the cull bits of apparel triangles;
- why the bare pelvis draws black in Exarust.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `apparel`: worn models, the underwear rule and its groin cover, painting
  by worn model.
- `player-movement`: the player's start outfit.
- `level-decoding`: placed characters wear exactly their stored items by
  worn model.

## Impact

- `exa_formats` (`objdb.rs`), `exa_character` (`lib.rs`), `exa_game`
  (`player.rs`), `exa_world` (`lib.rs`), `exarust` (`viewer.rs`,
  `main.rs`), `exa_tool` (`main.rs`, `corpus.rs`).
- Docs: `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/PLAYER.md`,
  `crates/exa_character/README.md`, `crates/exa_tool/README.md`.
