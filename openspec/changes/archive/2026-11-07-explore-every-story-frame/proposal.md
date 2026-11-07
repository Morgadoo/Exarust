## Why

PLAN Phase 2 (M2 walkable world), component TGridEnvManager, after round
117 made the story's exploration read the room manager. Its walks logged
one discovery and nothing more: the discovery pass ran once a second where
the original runs it every story frame, and the scripted walks barely left
their first cell. The item cursor's placement pick, the room manager's
last unwired occluder caller, placed items seen across a wall's hollow.

## Evidence

REA (Exanima.exe), read against the assembly:
- The experience unit installs its tick in the story's hook slot. The
  story's per-frame hook (playing phase; a dying player skips it) runs the
  wake-up branches, then a once-a-second gate (`now / 1000` above the last
  second) that runs only the pending level-task list, and then calls the
  tick slot every frame it gets that far, gated by nothing else.
- The tick explores, then converts the player and each companion; each
  conversion keeps its own clock (points are due per elapsed `cost × 50`
  ms), so a per-frame call converts as much as a per-second one.
- A cell earns its points once per level map: the bit is set when found
  and the save's discovery chunk keeps the maps.
- The item cursor's placement routine picks the scene under the cursor and,
  in a grid scene, asks the room manager's occluder segment query with the
  pick's origin, direction and hit distance; a blocked pick clears the
  placement flag and places nothing that frame.

## What Changes

- exa_game: the flow's story hook sends the discovery step every frame (the
  second gate only moves); a debug line names each cell the player enters
  with the map's explored count; the item cursor places nothing for a hit
  behind an active room's occluder.
- exa_world: `--toggle-door` toggles at the first frame from the asked one
  that a bound hinged leaf exists (door bodies register late).

## Out of scope

The pending level-task list behind the second gate.
