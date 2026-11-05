## Context

`exa_ai::search` is the search model. `exa_game::npc_search` hosts it in the running game, inside the follow state, with a scene kept across body ticks. Placed objects carry `exa_world::PlacedItem`, and `ObjectSections` when they are usable or containers. `container_fill` fills containers at world load. A body is dressed again whenever its `Outfit` or `Wielded` changes (`exa_character::refit`).

## Decisions

- **One scene, three lists.** A character keeps its place in the corpse list, and its object is that place's `object`. A placed object keeps its index in the object list. Indices are only appended, so the search's held object stays valid. A character or object that is gone loses its present bit. The containers are rebuilt from the sections on every refresh, so the player's popup edits are seen.
- **The box.** The object's place is its first node's `PartBounds` centre in world space, and its half-extents are those bounds' half-extents. Without bounds, the root's place is used with zero extents. The arrival radius is (sum of the half-extents) × 0.333 + 250, so the stand-in moves it little.
- **Lying in the world.** An object is searchable when its root has no parent or sits under a `LevelRoot`. That is Exarust's form of "the parent node is a sector": items in containers or hands are not entities.
- **Effects after the tick.** `follow_and_search` notes which present objects lost their present bit, which containers changed, and each body whose pack changed, together with its worn list and hands from before. `carry_out_search` then runs, chained after it in the brains' chain. It despawns the taken objects, removing their `ColliderNodes` and `StandIns` colliders first, as the player's take does. It writes the containers back by tagged id, matching each entry once. Last, it re-dresses the bodies.
- **Re-dress by difference.** Each id is matched once between the old and the new worn list. A garment whose model is the record's worn model leaves, and `garment_of(record)` is added, as the figure release does. Records come from `DollItems` (game ids) or `LevelLocalItems` (local ids), so a local id is never looked up in the game database. A corpse that lost a worn item gets `CharacterData::ensure_underwear` first. A changed hand recomputes `Hands` (keeping a carried torch) and inserts or removes `Wielded`.
- **Held entries stay listed.** The pack's carried list is the original's inventory, and a hand only references an entry. The re-arm weighs that list, so a wielded item must stay in it. Otherwise a second take could drop the first weapon. This corrects the model's earlier removal.
- **The debug recruit.** `--searcher` applies the recruitment's role edits by id, so a character whose Activity Wait role would override Explorer's follow (Derrin) follows. It is still a debug aid: there is no dialogue engine.

## Risks

- About 1900 placed objects in exanima01 are refreshed each searching body tick. Only searching followers pay for this.
- A gathered operative object, such as a lever or wood debris, is walked to and weighed as nothing, as in the original.
