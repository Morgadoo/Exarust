## Why

Roadmap task B4 (lane `pathfinder`), the open TPathFinder rows of the
function ledger and finding F388. Every live AI search passed size class 0,
penalty shift 0 and the one-cell accept distance, and never the caller's
end test, where the original's planner reads all four from a goal record
each AI state writes. A blocked line walk returned the raster point where
the original projects it onto the walk's direction.

## Evidence

REA (Exanima.exe) and the disassembly, checked against the instructions:

- The planner reads the size class, the penalty shift and the accept
  distance from the caller's goal record, and calls the record's end test
  when the search stops short (the 100 cm rule only without one).
- The AI constructor writes size class 3, shift 1, accept 10. The shift is
  the group-alert count the AI tick decays.
- The roam and patrol states write size class 4 before the wander and
  home-side picks, which write accept 20 and the flat end test at 150 cm.
- The chase and the flight write the roles' phase word plus 3; the chase
  writes accept 10 and the half-height test at 150 cm (300 cm for a downed
  target); the flight's way out writes accept 30 and the flat test.
- The follower's walk to the leader writes size 0, accept 50 and the
  half-height test; its closing walk size 4, and a moved goal accept 20
  with the test that never fails. The search writes size 7 and accept 50.
  The Long Door's query writes accept 50, size 3, alert count 1 and the
  test that never fails.
- The line walk's and the straight walk's block arm projects the backed-up
  cell point onto the start-to-end direction normalised with rsqrtss.
- The attach's seed byte comes from a global with no writer.

## What Changes

- `exa_world::nav::find_path_to` takes the search record and an end test.
- `exa_ai::route::PlanGoal` on each brain; `NavRouter` plans with it and
  the alert count; each planning state writes its fields.
- The line walk and the straight walk project a blocked end.

## Impact

- Crates: `exa_world` (nav path, query), `exa_ai` (route, brain, chase,
  walker, idle, lib), `exa_game` (npc_search, revive_pass, long_door_live).
- Lockstep traces change in scenes where NPCs plan routes (roam, patrol,
  chase, flight, follow, search) and where the player's steer or a duel
  line walk is blocked.
