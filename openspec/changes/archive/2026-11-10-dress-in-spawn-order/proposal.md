## Why

Round 154 (motilepart3) found the arena duel (`play arena --opponent --weapon ub05ssworda --frames 1500
--fixed-frames --trace-all`) bimodal in its values from the first weapon clash (about tick 325): one mode read the
clash at 53 cm and 10.0 cm/tick, the other at 55 cm and 11.5 cm/tick, keyed on the player's weapon's entity id
(2390 or 2381). The body solver, its pair stage and the sector grid walk the bodies in entity order (`step_bodies`,
`rebuild_sector_bodies`), and the pair stage lets the first-found part of a tie bring its spheres, so the order of
the two fighters' weapon bodies decides who pushes whom in the clash. Those bodies are spawned when the characters
are dressed, and the dress ran on an 8 ms frame budget with its garments' fits and paints on the async pool: which
character was dressed first, and so whose weapon took the lower id, hung on load timing. A second load-timing id
was found on the way: `NavMovers`, a bridged resource only the fixed step names, was not registered in the frame's
world by the second update, so the frame's resource hold missed it and its insert (when the navigation grid's
async build finished) took an id before or after the characters' (body ids 1 apart between runs). docs/PLAN.md
Phase 3, the simulation thread.

## What Changes

- `exa_character::DressAtOnce` (inserted by `exarust` under `--fixed-frames`): the dress takes every body in the
  frame its parts are attached, with no frame budget, building the garments' fits and paints there (or waiting for
  those already on the pool); the hair pass drops its budget too.
- The dress, the hair pass and the off-hand pass walk their bodies in spawn order (the entity index, ascending)
  instead of the storage's, in every mode.
- `exa_physics::sim_bridge`: bridging a resource registers its type in the frame's world when the plugin builds
  (`BridgeHalf::register`), so the frame's resource hold at the second update holds its entity.

Rule change: none. Every lockstep run now draws the 55 cm, 11.5 cm/tick mode: the dress runs the player first, so
the player's weapon takes the lower id (as in round 152's three runs). Free mode keeps the budgeted dress.

## What the original does

The original creates a character with its equipment in one go and adds each body to its sector's list as it is
created, so its items take places in the list in the characters' creation order. The sector step's walk is not
re-traced here (round 154 notes it walks the list from its end).

## Out of scope

- Bevy 0.20's `Ord` on `Entity` compares the stored bits, which hold the index inverted: `sort_by_key(|e| *e)`
  (`step_bodies`, `rebuild_sector_bodies`) runs newest first, not in spawn order as their comments say. The
  solver's order is left as it is (it would move every trace); whether the original's pair walk runs from the
  list's end for bodies too is to be checked against the assembly.
- exanima01: every attempt died on a GPU Out of Memory (an external vLLM engine holds 18 GB of the 3090).
