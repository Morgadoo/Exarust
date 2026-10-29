## Context

`exa_combat::attack::execute` is the executor that the player and the duellist share. `exa_ai::search` is the search model: its tick, its gathering and its deeds over an abstract scene. The running game keeps a character's items in `exa_items::possession::Possessions` on the body model, and the dead bodies keep the corpse condition word. Item records live in the game's object database. No format page changes (the item sections are those of `docs/formats/` object database notes).

## Decisions

- **The held count is the wielder's blade count.** The original keeps a held list. Its empty hands' set-up puts both hand bodies in it (count 2), and the empty-hands reset alone leaves it at 0. Exarust spawns the fists as two blades and a weapon as one, so the blade count matches the original's count in every live case. The guard is a signed test of at least 1, made before the script veto and the busy test.
- **One scene, kept across ticks.** The search holds objects by their place in the scene's lists. A character therefore keeps its place once it has one. Its object moves with it and its pack follows its possessions every body tick. A character that has left the world loses its present bit and its character mark, so it is never gathered again. The connected flags are built once for a level, from its room table. This is a stand-in: the original's room grid keeps its own connected bit.
- **Deeds on the model, then written back.** Each deed is carried out by the search's own `apply_deed` on the model packs. The changed packs are then written back to `Possessions`, keeping the flags and places of the entries that stayed.
- **The searcher's answers.** Reachable is the level's sight callback, which is what the original's pick asks. A path is the navigation grid's route on the follow's own planning turn, and a straight line without a grid. The attention is on someone else once the attended contact's notice score reaches 1; otherwise it is on the goal within 12 m, the goal entity's reach. The alert look takes the tick under the idle head's own gate.
- **The follow hosts the search.** This is the original's follow state through `exa_ai::idle::follow_leader`, with the player as the leader. The search's answer is the follow's busy flag, which widens the band to 1200 cm.

## Open questions

- The original's room-grid connected bit: what sets it, and whether a door shut between rooms clears it. To be answered with REA on the room grid's builder.
- The corpse object's box. The arrival radius uses a lying body's stand-in half-extents. To be answered by reading the object's bound in the scene unit.

## Risks

- A brain whose roles choose the follow now walks after the player. Before this change it stood. No placed character of the shipped levels starts in the follow; only the debug flag and future scripts put one there.
- The executor now refuses a body with no blades, such as a race without hand bones. The fists of every human-frame fighter are two blades, so unarmed fights still start.
