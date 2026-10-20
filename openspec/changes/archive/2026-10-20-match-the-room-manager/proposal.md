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
- Evidence: `0x100142f80` (cell centre), `0x100143000` (room at a point), `0x100141840` (flood), `0x1001417b0` (side test), `0x100143290`/`0x1001436f0` (linked and door reach), `0x100144610` (fade), `0x100143c20` (tracker), `0x100142110` (room bounds and portals), and the helpers `0x100140520` (edge rotation) and `0x100142cd0` (direction step). The constants are `0x10031a2f8` (1/600), `0x10031a2e0` (600), `0x10031a2d0` (280) and `0x1002ba840` (opposite sides). No new corpus files.
- Out of scope: the room render objects and buffers, the partner-room mask, the migration payload, descriptor copies, segment tests and object attachment.

## Phase

Phase 7, world. The room manager's exact conversions.
