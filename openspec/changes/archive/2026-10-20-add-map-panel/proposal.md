# Proposal

## Why

Testing a fix across levels meant quitting and relaunching Exarust from a
terminal with another level name, and the view switches (shadows, every
room, the navigation grid, the body overlay) were only flags or scattered
function keys. A panel in the window lists the game's levels and the map's
view switches in one place.

## What Changes

- F9 opens and closes a map panel in the level and play views.
- The panel shows the current map and switches for point shadows, every
  room, the navigation grid and the body overlay, which take effect at
  once.
- It lists every level in `Resource.rpk` (a chunk tree with a tile grid or
  placed props, found from the top-level chunk headers only). Choosing one
  restarts Exarust on that level in the same view, keeping the other flags
  and dropping the ones tied to a place or a script (`--at`, `--camera`,
  `--screenshot`, `--press`, the pauses, …). On Unix the process replaces
  itself; elsewhere it starts the new one and exits.
- While the panel is open the world cursor is hidden, the system pointer is
  shown, and mouse buttons and the wheel go to the panel, not the game.
- Cursor Look no longer leaves the system pointer visible: the cursor draw
  set it back to visible while the mouse was captured.

## Capabilities

### Modified Capabilities

- `world-viewer`: the map panel.

## Impact

- `exarust::map_panel` (new), `exarust::main` (plugin, the arguments kept
  for a restart).
- `exa_formats::level::is_level`.
- `exa_ui::cursor`: `PanelPointer` and the pointer visibility rule.
- `docs/DEBUGGING.md` (Map panel), `docs/UI.md` (World cursor).
- Evidence: none needed (Exarust's own tool); the level test matches what
  `exa-tool corpus levels` counts as a level.
- Out of scope: switching levels without a restart (the world, physics,
  GI and navigation are built once per process), the original's own level
  transitions, and settings that only apply at load (GI, supersampling).

## Phase

Phase 8, tooling for testing (debug tools).
