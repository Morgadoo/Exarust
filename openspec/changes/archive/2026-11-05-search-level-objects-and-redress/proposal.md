## Why

Backlog P7. The companion's search ran in the game over dead bodies only. The original's search also gathers the containers and loose items of the nearby rooms, and the body it changes is dressed again. Two parts of the backlog entry did not hold up against the original. The original's search plays no kneel clip. The running game does have searchable object instances: every placed object of the level.

Phase 6 (AI) in docs/PLAN.md: TAISearchActivity.

## What the original does

- **Gathering.** The gather walks two lists in each flooded room cell: the room grid's body list and its static-collider list. From either list it takes an object whose class word is the placed-object kind (`& 0x7000 == 0x4000`) and whose parent node is a sector (class bit `0x10`, that is, it lies in the world). The object must not carry bit `0x8000` in its placement word and must not be searched, and its record must have an apparel, weapon, operative or container part (`0x35000`). From the body list it also takes a dead character (state `0xA00`).
- **Arrival.** A record with a container section is looted from its contents. Any other object is a loose item: it is weighed itself and taken last. A take of a loose item removes its node from the scene.
- **The settle.** On arrival the search calls the brain's settle and attend routines. These stand the body in the idle motion (`0x13`) and turn the head; the waits after them do the same. The search never asks for the motion set's `Kneel` action (`0x20`). No routine of the search unit starts a motion of its own.
- **Wear.** The wear job runs the equip conflict pass. A worn piece that overlaps and has a model byte over 50 refuses the wear. Otherwise every overlapping piece comes off into the carried list, except the underwear, which is discarded. The job then puts the item on, refits the searcher, and only then takes the item from its source.
- **A take from a corpse.** Taking a corpse's worn item removes it from the worn list, runs the underwear rule, and refits the corpse. A gripped item is released instead.
- **Hands.** Wielding marks the item's inventory entry held and takes it in hand through the hand job; the inventory keeps the item. Stowing sets the stowed slot.
- **Who searches.** Only Explorer (role 31) creates the search (idle activity 6), and only with its Activity variable set to the follow. No placed character of the shipped levels holds Explorer. Derrin, the story companion of exanima01, gets it from his recruitment topics ("Come With", and the line "We should look around. My Da would say just a good shirt can save your skin"). Those topics remove roles `0x14` and `0x21` (Activity Wait), add roles `0x13` and `0x1F`, and set `Role($1F).Activity = 4`.

## What Changes

- The running game's search scene lists the level's placed objects with a model, besides the characters. For each it keeps the box centre and half-extents, the tagged id, whether it lies in the world, and the placement bit. A container's contents come from its object sections. The level's own records are weighed under their tagged local ids, and the size bytes for drops are read too.
- The deeds' world edits are carried out:
  - a taken loose object leaves the world with its colliders;
  - a changed container is written back into its sections;
  - a body whose worn list or hands changed is dressed again. Its outfit loses and gains garments, which asks for a refit. A corpse gets the underwear rule first. A changed hand updates the hands and the weapon model.
- Search model (exa_ai):
  - wearing runs the equip conflict pass;
  - wielding keeps the item listed and marks it held, and clears the old main item's mark;
  - stowing no longer takes the item out of the list;
  - the arrival state is now `Settle` instead of `Kneel`.
- `--searcher <id>` applies the recruitment's role edits as well as Explorer, so Derrin (`0x2b`) searches as he does once recruited.
- `--searcher-shots <dir>` saves a screenshot at each pick, settle, loot, take and re-dress.

## Out of scope

- The original's own body and static-collider lists. Exarust has none (S10), so every placed object with a model stands in for them, and the model-less volumes and placeholders are left out.
- The `0x8000` bit of a loose item that has no object sections. Exarust keeps the placement word only for usable objects and containers.
- The wear job's call on the worn item's body state (a 0.3 factor; not identified).
- The stow's move of the old stowed item to the second stowed slot.
- A take from an untagged, game-database container: the original's removal does nothing there.
- The kneel: the original never plays it.
