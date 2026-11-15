## Why

Roadmap task B1 (wiring lane `wirechardata`), findings F246, F247, F248 and
F251 of the characters-06 audit. Many rules of the character record had no
live caller: the live inventory, trade and AI paths did their jobs with
their own code, some of it different from the original. The skill-ready
alert never showed, the inventory's pack ignored the worn bulk, a hand item
dropped out of the inventory left the other set holding it, a unique item
sold in the arena stayed in the members' outfits, an item an NPC picked up
was listed at the wrong cell, and level-placed characters ignored their
saved wounds.

## Evidence

REA (Exanima.exe), checked against the assembly where order or constants
matter:

- The inventory's fill binds the pack (fill, clamp), then adds the worn
  weight rule's sum (each worn item's bulk) to the pack's used capacity.
- The inventory's drop callback, for a drag that did not start in the pack
  and ends outside the view and the pack, takes the last carried copy
  marked `0x100` off the record, refills the pack and drops the record's
  hand and loadout references past the copies owned.
- The arena's sold hook, for a unique item, forgets it everywhere on the
  manager's record and on each employee's record (inventory, worn list,
  both hand pairs, staged list, loadouts).
- The record's append writes the third word as `0x28, 4`.
- The import spawn builder writes `max × (stored × 1/65535)` (double
  precision) into the body's yellow and red after the body is built.
- The skill selector's unlearn (the record's cascade) is reached only from
  a closed cycle of the selector's own windows; the live skill screen ports
  its own routine with its own cascade.

## What Changes

- `hud::pick_status` reads the player's record for the skill-ready alert.
- `body_inventory::bind_inventory_pack` adds the worn bulk at the
  inventory's bind; `body_inventory::with_record_items` runs a record rule
  on live possessions.
- `figure_pick::release_figure` runs `take_equipped` and
  `drop_surplus_refs` for a hand item dropped outside the player's pack.
- `arsenal::Holder::outfit`: the members' and the manager's outfit lose a
  unique sold item.
- `npc_search::sync_possessions` lists new carried entries at `0x28, 4`.
- `save_live::restore_placed_pools`, called by the viewer's placed spawn.
