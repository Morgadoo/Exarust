# Proposal

## Why

Phase 6 (AI) of docs/PLAN.md. A DeepSeek lane extended the companion's
search activity, the pathfinder's line tests and nearest-walkable point,
and the role unit's script functions, and bridged the AI's encounter
reports into the script engine. Review against the assembly kept the true
rules and reversed the invented ones.

Evidence (REA and the objdump listing of `Exanima.exe`): the four line
helpers' loops (counter from -1, continue while below count - 1, so one
step per cell of the major span, the end cell included) and their major
axis choice (`COMISS |dz|, |dx|`, Z on a tie or unordered); the
nearest-walkable off-floor search (the squared term added to each key is
a register the routine never writes before that point, so the ground key
stays); the generic section accessor's kind table (1, 2, 4, 8, 0x10,
0x40, no 0x20); the search tick's approach state (the walking attention
of 5.1 jumps to the answer with busy still set); the worn take (the grip
release clears the hand word after the first hand's id and flags) and the
carried take (clears the fourth hand word); the search's twenty phrase
strings in the image; the encounter routine (meeting time renewed before
the scripts run); `StartDialogue` (the first argument read as the topic).

## What Changes

- Line walk, straight walk, obstacle and the planner's budget line walk
  walk the asked end's cell and take Z as the major axis on a tie.
- The search's phrase tables are the original's strings; a carried take
  clears the fourth hand; small object edits are named functions.
- `exa_formats::roles` documents why the condition after a corpse and a
  0x10 section is the block after the 0x10 one.
- `exa_game::role_scripts` runs encounter scripts through `exa_script`
  and renews the cast relation's meeting time; `Conversations::with_scene`
  replaces three copies of the scene setup.
- `exa_ai::roles::start_dialogue` carries the topic; `migrate_former` and
  `first_invalid` name existing steps; `queue_box` marks a node's box dirty.

## Out of scope

- The deferred character-view jobs (take, wear, drop run immediately).
- `StartDialogue` from the AI side (the live call is `exa_script`'s
  builtin); interaction conditions run by the player's use.
- The grip object itself (the second hand slot stands in for it).
- Wiring `queue_box` and `start_full_refresh` into the running game.
