# Proposal

## Why

docs/PLAN.md, Phase 7 (game modes and content), arena paragraphs (arena3,
wire-arena-hub, wire-hub-pick): the match end, the next round, the return to
the hub and the hub's shopkeeper and specialist placements were tested rules
that the game could not run, because nothing built an arena fighter, a
company entrant or a hub NPC from a spawn plan, and a hub match restarted the
process without its career.

## What Changes

- `exa_game::scene_bodies` (`SceneBodiesPlugin`): a scene body generator
  that builds a body from a `SceneBodyPlan` (generic, hire-list, Beast,
  company and hub plans) on the placed characters' spawn path (morphed body
  model, dressed garments, skin look, held item, `Physical` for the ragdoll,
  record data, roles, brain) and removes it, capsule and subtree, by its root
  or its body model. The game's tables are parsed once, on the first command.
- `exa_game::arena_live` (`ArenaLivePlugin`): stands the first round's bodies
  from the match layout, runs the end screen's Continue (Enter/Space, a
  stand-in for the undrawn screen), the next round (winner stays, the reserve
  or a fresh opponent after the round's yield) and the return (settles the
  match into the career and asks for the hub).
- `exa_game::arena_handoff`: the career and match setup cross the process
  restart in a versioned file; the in-process level change stays blocked
  (`level_transition`).
- The hub asks for the shopkeeper (`arenashpkp`) and one body per employed
  merchant/physician at their planned spots, and places a returned manager at
  the saved placement.
- Review fixes: the generator no longer parses the object database every
  frame; a despawn by body model removes the root and its capsule; the hub
  entry also runs under the play view's practice stand-in mode (the manager
  was never placed and no hub body asked for, also on main); the return no
  longer turns the player root on top of the body's facing; the viewer and
  the generator share one copy of the body helpers.

## Evidence

The arena's generic and hire-list spawn routines (record generated at stat
level 5/6, profile bytes `[3, tier, 100, 10, 10, …]`, the AI clock set to now
+ 3000 ms on both action words), the tournament round's spawn call (first
argument the event's gear tier), the hub's shopkeeper and specialist
placements and the saved return placement (300 cm, half turn, flag 0x100),
read from the assembly and the arena pack. GPU check: `play arena1
--opponent` still starts a duel; `play arenahub` loads in the same time as
main and the shopkeeper stands behind the counter.

## Out of scope

The generated fighter's own record and outfit (actor templates stand in), the
hire-list opponent's forced company outfit, the Beast's body model, the end
screen and forfeit button, the bracket's next fight, an in-process level
change, the manager's saved record as the match body.
