# Proposal

## Why

docs/PLAN.md, Phase 7 (game modes and content), arena paragraphs
(wire-arena-bodies, wire-level-reload): the live match ran its rounds and its
return, but the end screen was not drawn (Continue was an Enter/Space
stand-in), there was no forfeit, a tournament's Continue did not raise the
bracket's next fight, and generated opponents took practice actor templates
instead of their own generated records and outfits.

## What Changes

- `exa_game::arena_live::ArenaEndScreen`: the round result opens the arena's
  single-row end menu through the GUI draw path (`NEXT FIGHT` on screen 3,
  `EXIT;` on screen 2, scale 0.9); Enter or a pointer release over the row is
  the Continue. Escape opens the arena pause page; its FORFEIT row
  (`flow::ForfeitMatch`) settles the match as forfeited and returns to the
  hub, and a dead manager's forfeit is ignored.
- A tournament's Continue resolves `arena_session::BracketState` and raises
  the next fight with `match_layout::raise_fresh_match`: the first surviving
  teams, as many as the event has sides; the event weapon row travels with
  the in-process launch (`MatchSetup::weapon_variants`).
- `exa_game::arena_records`: a generic or hire-list opponent writes its own
  character record (random NPC pass, professions, name, look, skill rows),
  personality −100, 100, 0, and its outfit (level pass, or the hire profile
  with the forced company bytes and the event weapon); `scene_bodies` dresses
  the merged character mesh from it. `objdb::Item` exposes the rank word and
  armour type; `ItemFacts::of_item` reads them. The NPC and outfit passes draw
  their unit values from the same sequence as their high-word draws.
- Debug: `--arena-fell <frame>` drops every arena opponent dead so the end
  screen can be checked unattended; `--press` names `Enter`.
- Review fixes: the next fight formed every surviving team into one ring and
  overrode the raised role flag (a duel event became a free-for-all); a
  hire-list opponent wrote no personality; the world crate's test item did not
  compile; the hand-off file is gone after the rebase onto the in-process
  level change.

## Evidence

The arena's end routine (Continue branches on the round count, the
tournament's raise or the next-round routine), the forfeit routine, the
tournament raise (its team walk stops at the event's side count; role flag
0x12 below three teams, else 0x10), the generic and hire spawn routines
(reseed, NPC pass at level 5/6, outfit pass, personality −100/100/0 from the
same constants in both), the engine's float randomiser (steps the global seed,
scales by 2^-32 in double, narrows once), the strings `NEXT FIGHT`, `EXIT;`
and `FORFEIT`, read from the assembly. GPU check on the RTX 3090: `play
arena1 --opponent` starts the duel with a generated, dressed opponent; the
end screen draws when the round ends and its Continue changes the level to
the hub in process; `play arenahub` loads as before.

## Out of scope

The Beast's body item, the company entrants' and hub specialists' saved
records, the drawn rival company per team (the team id stands in), the clock
reseed before each generated body (the arena roll continues), the item
instance's generated quality and condition, the score board, round banner and
per-fighter condition bars.
