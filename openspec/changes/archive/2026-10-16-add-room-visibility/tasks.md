## 1. Rooms

- [x] 1.1 `exa_world::rooms`: `Rooms::flood` (seed, door stack, link test, rectangles), `cell_of` rounding down, `room_at`, cell centres, door neighbours
- [x] 1.2 `RoomTracker`: wanted rooms (door point a quarter of the way to the neighbour's centre, 2 m), the ordered list, enter/leave/turn-around with mirrored start times, sticky states, the overlay alpha and its switch below 0.01, the debug view of every room
- [x] 1.3 Unit tests on synthetic grids: seed row, walls and doors, links only to open cells not walled against the door, closed cells joining without spreading, islands, cells without a wall set, rounding down, entering and turning active, the 1.99/2.01 m door reach, leaving and dropping to adjacent, continuous turn-arounds, an anchor without a room, sticky states

## 2. Shells

- [x] 2.1 `exa_render::shell`: `ShellMaterial`, opaque unlit black, and the overlay variant (blended, depth compare Always, no depth write, sorted last); never casts shadows
- [x] 2.2 `exa_world::grid`: `[…` children merged per room into the occluder, `+R…` children and the ceiling slot pieces into the lid; nothing of them spawned; ceiling caps from the first wall set; occluder, lid and overlay entities per room under their own root (no collision); grid meshes tagged with their room; only top-level set nodes indexed as pieces; `--ceilings` removed

## 3. Room view

- [x] 3.1 `exa_world::room_view`: `LevelRooms` at the grid stage; `RoomAnchor`, `InRoom`, `RoomRoot`, `PendingRoom`, `RoomTracked`, `RoomShell`; the tracker each frame on real time; hidden rooms' meshes hidden; occluders, lids and overlays shown by state and fade
- [x] 3.2 Rooms of props under the level root, placed objects and the level model's top nodes from their meshes' world bounds; meshes added later take their root's room; characters hidden by the room of their position
- [x] 3.3 `exarust`: the anchor on the player, and 20 m ahead of the fly camera in the level view; the level overview and `--all-rooms` show every room; usage text

## 4. Terrain

- [x] 4.1 `terrain::grid_blocks`: block presence from plane A `0x800000` with the grid offset; quads, skirt and plants of absent blocks dropped; one mesh per block room under the tile, sharing the tile's material; tiles without a block not built

## 5. Verification

- [x] 5.1 Corpus (skipped without `EXANIMA_DIR`): arenahub 10 rooms, 18 links, the live room numbers and rectangles, active rooms {2, 1, 7}, {2, 1}, {1} at the d1, c3 and b9 anchors and the live adjacent/hidden states; exanima01 153 rooms, 356 links, no open cell without a room; every arenahub and exanima01 room cell (special tiles aside) covered from above by its shells with up-facing triangles
- [x] 5.2 In game through `gpu_run.sh --gpu 4000`: the c3, d1 and b9 retail poses in play mode against the retail shots and main; d1 in the level view; a walk through the bar's south door (the overlay mid-fade); `play arena`, `play arenao3`, `level arenao3`, `play exanima01` at the start and the retail camera, `level exanima01` overview; no validation error from the change
- [x] 5.3 Remove probes from the tracked tree

## 6. Documentation

- [x] 6.1 `docs/RENDERING.md` (Rooms and black fill; Not done yet), `docs/formats/RFC.md` (Grid building), `docs/formats/TERRAIN.md`, `docs/PLAN.md` (Phase 2), `docs/COMPONENT-MAP.md`, `exa_world` and `exa_render` READMEs
- [x] 6.2 `openspec validate --all --strict`
- [x] 6.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
