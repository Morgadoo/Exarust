# Proposal

## Why

`TGridEnvManager` (15.1 KB) is the room half of the level's sector grid: the open-cell flood, the door links, the room AABBs and the active-room tracker. `crates/exa_world/src/rooms.rs` and `room_view.rs` already reproduce most of it, but the component is rated 0 on stale evidence, and two exact conversions and three helpers were missing.

## What Changes

- The world-to-cell conversion multiplies by the original's `f32 1/600` (`0x3ADA740E`) instead of dividing by six metres.
- A room's cell rectangle converts to world bounds in centimetres, `[xmin, xmax, zmin, zmax]`, with the 0–280 cm ceiling band.
- The packed edge word rotates by quarter turns, keeping the high nibbles' bit 3.
- A direction vector gives the original's opposite one-cell step, using `atan2(x, z)` and the four strict quadrants.
- The component is re-rated from the room flood, portal sizes, door reach, active-room tracker, fades and shells already shipped in `rooms.rs` and `room_view.rs`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `room-visibility`: the room manager's cell and edge conversions.

## Impact

- `exa_world::rooms`, from `INV_CELL_CM`, `CELL_CM`, `ROOM_CEILING_CM`, `rect_world_cm`, `rotate_edge_word` and `cell_step_from_dir`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TGridEnvManager` becomes **45%** in the source rating. The headline rises.
- Evidence: the room manager's cell centre, room at a point, flood, side test, linked and door reach, fade, tracker, room bounds and portals, and the edge-rotation and direction-step helpers. The constants are 1/600, 600, 280 and the opposite-side table. No new corpus files.
- Out of scope: the room render objects and buffers, the partner-room mask, the migration payload, descriptor copies, segment tests and object attachment.

## Phase

Phase 7, world. The room manager's exact conversions.
