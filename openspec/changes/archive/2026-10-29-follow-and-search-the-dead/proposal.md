## Why

The AI action executor started an attack whatever the body held, where the original first tests the body's held count; and the companion's search (idle activity 6) was ported and tested but never ticked in the running game, which now has dead bodies with possessions, loot windows and the item database to search them with.

Phase 6 (AI) in docs/PLAN.md: the NPC AI notes' executor and search entries (SG_CharacterAI and TAISearchActivity).

## What Changes

- The attack executor refuses to start anything while the fighter holds no striker. The held count counts the fists: the empty hands' set-up holds both hands (2), one weapon is 1 and two weapons 2. Only a body with no hand blades at all is refused. Both the AI's readiness and the player's attack pass the wielder's blade count.
- The search reads an item record as its weighing reads it. Apparel comes first: the occupancy, coverage, protection and special bytes, plus the model byte. A weapon comes next: its flags, quality, damage, thrust, tier byte and special word. Last, an operative or container part makes the item worth gathering.
- A level brain whose roles choose the follow follows the player in the running game. Past its band it plans through the navigation grid on its planning turn. Inside the band it runs the search anchored on the player, over the level's characters: the dead bodies are its candidates, with their possessions as packs. The deeds are carried out on both sides' possessions.
- `--searcher <record id>` gives a level character the Explorer role with its Activity variable set to the follow. This is a debug aid, because Exarust has no dialogue to recruit a companion.

## Evidence

The executor's first instructions and the empty hands' set-up, read in the disassembly. The search's item value routines and pick, the item description routine's weapon value, and the follow state, all read in the ai2 and motileai packs. The section offsets were checked against the object database's parser.

## Out of scope

The executor's cached cast-hook answer. The follow's close step and obstacle nudge. Level containers and loose items, because the running game has no searchable object instances. Level-local item ids, the kneel clip, and re-dressing a corpse or the searcher after a take. The comment's spoken bark, because there is no dialogue engine.
