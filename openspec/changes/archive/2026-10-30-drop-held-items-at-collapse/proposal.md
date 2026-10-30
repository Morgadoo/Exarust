# Proposal

## Why

The undead rise logged "unarmed set-up false" on the GPU
(`--undead-opponent 120 --still-opponent`): the risen duellist's hands
still counted as holding its sword. A collapse let go of the item bodies
(the grip release) but never told the holder's records: the hands the
moveset and the rise read kept the weapon, and the possessions record kept
it in its out set. In the original, every dropped held item runs the
holder's came-loose hook, which empties every non-shield hand of the
record's out set (and removes the item's held entry from the carried
list), and the revive's unarmed set-up tests exactly those two out-set
hands. Advances Phase 5 (combat), the motile part's collapse and revive.

Evidence: REA (Ghidra) on Exanima.exe 0.9.5: the revive's test of the
record's two out-set ids, the drop routine calling the holder's
came-loose hook, and that hook clearing the non-shield out hands; the
retail corpses lying beside their weapons (combat-look captures, held
count 0 at the collapse).

## What Changes

- `exa_character`: `Hands::let_go` (every hand but a shield's empties) and
  `Hands::is_empty`; `drop_collapsed_hands` runs on the tick a collapse
  starts, after the collapse words, so an armed body's emptied hands run
  the equip refresh (fists halved, weapon bit) through the hands' change.
- `exa_game::equip`: `forget_collapsed_hands` runs the came-loose hook on
  the possessions record of a collapsing body
  (`Possessions::forget_loose`, a shield by the catalogue's kind).
- `exa_game::rise`: the unarmed set-up reads the record's out set
  (`weapon_slots_empty`), the hands only for a body without a record
  (a shield or an off-hand light counts as held); the rise drops the
  collapse record with the death clock, so a second collapse starts afresh.

## Out of scope

The combat-mode byte the revive clears (Exarust has no Tab combat mode on
an AI body), the glow fade, the per-limb node-turn copy, a grabbed
object's release at the collapse.

## Impact

`exa_character`, `exa_game`.
