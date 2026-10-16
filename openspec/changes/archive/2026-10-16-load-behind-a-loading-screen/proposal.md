## Why

`exarust level` and `exarust play` built the whole level in one Startup
system, and play mode then dressed every character in one frame. The window
answers the desktop only between frames, so it stopped answering for tens
of seconds (about 25 s for exanima01) and GNOME offered to force-quit it,
naming it "Unknown" because the window had no app id.

Phase 2 (Static world), milestone M2 "walkable world": load any arena or
story level. The plan lists async work on Bevy's task pools for this phase.

## What Changes

- A level is built a part at a time behind a loading screen, about 25 ms of
  building a frame: the grid's sets and rows of cells, the terrain a tile or
  a palette material at a time, the level's own model, props, placed
  objects, then the lights. Work that cannot be cut into short pieces runs
  on the async compute pool: terrain surface maps, static colliders, garment
  fitting and painting, the GI source and the GI block.
- Play mode's loading screen stays up while the level's collision is built,
  the player and the level's characters are spawned and dressed, and the GI
  block around the starting view is built and warmed up, so the first frame
  shown is lit and every body is dressed.
- The GI warm-up runs 16 steps a frame instead of all at once.
- Every frame of the load and of the 10 s after it is timed; a frame over
  250 ms is logged with its stage.
- `--frames`, `--shove` and `--autowalk` count from the end of the load.
- The window's app id is `exarust`.

Evidence: none from the original is needed; the loading screen is
Exarust's own. The checks are runs against the installed game: frame times
from the log, screenshots of the first frame after the load, and the
Wayland protocol log for the app id.

Out of scope:
- the original's own loading screen and its look;
- texture streaming with placeholders (`docs/PERFORMANCE.md`);
- the GPU's cost of the first frames drawn with 4× supersampling and of the
  first frame drawing a crowd with shadows;
- shortening play mode's load (exanima01 takes about 15 s, most of it
  dressing 39 characters while the scene renders behind the screen).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `user-interface`: the loading screen and the long-frame log.
- `world-viewer`: levels load over frames, scripted runs count from the end
  of the load, the window's app id.

## Impact

- `exa_world` (`lib.rs`, `grid.rs`, `terrain/mod.rs`, `gi_volume.rs`),
  `exa_physics` (`lib.rs`), `exa_assets` (`model.rs`), `exa_character`
  (`lib.rs`), `exa_render` (`gi.rs`), `exa_ui` (`loading.rs`), `exarust`
  (`main.rs`, `viewer.rs`).
- Docs: `docs/UI.md`, `docs/RENDERING.md`, `docs/PLAN.md`,
  `crates/exa_world/README.md`, `crates/exa_character/README.md`.
