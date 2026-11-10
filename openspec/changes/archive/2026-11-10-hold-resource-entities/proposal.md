## Why

After round 149 the arena duel with presses (`play arena --opponent --press D@200-260,W@400-460,S@600-700 --frames
3000 --fixed-frames --trace-all`) still gave two or three traces across runs of one binary. The values never differed
(every r151 trace and the older mode A have the same rows with the `body` column cut, md5 d9879229...): only the body
ids did, by 1 or 2, from tick 1. In Bevy 0.20 a resource is an entity, spawned the first time the resource is
inserted. The navigation grid (`NavGrid`, `NavMovers`) is inserted when its async build finishes, which falls before
the characters' spawn in some runs and after it in others; the low field (`SceneLowField`) is inserted once the camera
has a size, before or after the level's walls. Each such insert takes the next id from the allocator in the middle of
the level's or the characters' spawns and moves every id after it. The startup's parallel systems also number their
spawns (the GUI's nodes, the camera, the level's root) in thread order. docs/PLAN.md Phase 3, the simulation thread.

Evidence: `EXA_ENTITY_TRACE=1` (new) on four identical 150-frame duels: three runs put `NavGrid`/`NavMovers` at ids
2206-2207 before the player (2208), one after the characters (player 2206): the body ids 2 apart.

## What Changes

- `exa_physics::resource_hold`: `ResourceHoldPlugin` (added by `SimBridgePlugin`) spawns an empty entity (only
  `IsResource`) for every registered resource type at the start of the frame's second update, when the first update
  has built every main schedule; a later insert lands on it and a removal keeps it. The simulation's world holds its
  bridged resources' entities when the bridge starts (`BridgeHalf::hold`), after the frame's newly reserved ids are
  in its allocator.
- Under `--fixed-frames` the startup schedules run their systems one at a time.
- `EXA_ENTITY_TRACE=1`: logs each entity of the frame's world the first frame it is seen, with its components.
- `context/tools/determinism_grid.sh`: `GRID_ARGS` passes grid options (a lower `--gpu-need` while another program
  holds the GPU); a missing trace now fails instead of counting as a match.

Rule change: none. The rows are those of every earlier mode (values identical); the body ids are new ones (the held
entities come first), the same in every run: 4294964954 and 4294964993 in the duel.

## Out of scope

- Entities spawned while the characters are dressed (garments, held weapons): the dressing runs on a wall-clock
  budget and paint tasks, so their ids still interleave by load timing (seen in one of four duels, from id 2380 on,
  with the same trace). The load waits for every body to be dressed and built before the first tick.
- Free mode, which is not tick-identical by design.
