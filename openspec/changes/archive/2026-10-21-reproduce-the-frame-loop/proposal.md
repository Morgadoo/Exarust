# Proposal

## Why

The original's platform and frame layers (the frame tick and its pacing,
clocks, timer table and deferred calls, the logger's presentation, window
and display modes, the renderer's pass order and the 3D view's projection,
viewports and super-sampling) were reproduced only where Bevy and winit
already do the same thing. `RF_OS_Functions` stood at 34%, `RF_Main` at
30%, `FLX_System` at 51%, `TRenderer` at 35% and `RF_3DView` at 60%. A
first pass added rule models for them; review against the instructions
found several rules wrong (full screen treated as exclusive, the render
scale tied to the super-sampling level, the region stack placed from the
viewport, the deferred ring's capacity and wrap, the overlay's expiry, the
idle wait, the scene pass's order) and held keys not released on a focus
change.

## What Changes

- `exa_game::frame`: the frame-rate cap (`FramePacer`), the real and game
  clocks with the paused-time accumulator, the redraw-flag idle wait, the
  periodic timer table, the 128-slot deferred-call ring, the status history,
  the timed overlay and the logger's presentation levels, as the tick
  computes them.
- `exa_game::window`: the display-mode list, the refresh rate (unknown below
  30 Hz), window fitting, the 640 × 480 minimum, the full-screen rule
  (borderless over the desktop, exclusive only for a listed
  `ScreenWidth`/`ScreenHeight` other than the desktop's), the swap
  interval's present mode and the Alt+Enter toggle.
- `exa_game::hw_config` applies the full-screen rule; the settings'
  `FPSLimit` and `--max-fps` feed the cap, off otherwise.
- `exarust::frame_loop`: the cap from the previous frame's start, the
  focus-change release of held keys and buttons, Alt+Enter; a background
  window runs on winit's low-power 32 ms update.
- `exa_render::passes`: the scene pass's order with its depth states and
  the decal depth bias, the blended and opaque list orders.
- `exa_render::view`: the super-sampling level table and jitters, the
  separate render-scale mode, the projection, point projection, the
  rectangle clip, the framebuffer viewport and region stacks and the view
  copy.

## Capabilities

### New Capabilities

- `frame-loop`: pacing, clocks, idle wait, timers, deferred calls, log
  presentation, window and display modes, focus.

### Modified Capabilities

- `rendering`: pass order, list orders, view render size, projection and
  framebuffer stacks.

## Impact

- New modules `exa_game::{frame, window}`, `exa_render::{passes, view}`,
  `exarust::frame_loop`; `HardwareConfig::screen_size`;
  `Options::max_fps`.
- Docs: `docs/PERFORMANCE.md` (Frame pacing), `docs/RENDERING.md`
  (Supersampling, Pass order), `docs/UI.md` (Settings options), the three
  crate READMEs.
- Evidence: REA/Ghidra on the frame tick, its pacing routine, the vsync
  setter, the timer table, the deferred ring, the sleeps, the logger, the
  status line and overlay, the window procedure, window creation, the
  display-mode list and refresh query, window fitting, the full-screen
  toggle, the framebuffer viewport and region routines and the rectangle
  clip, the renderer's list sorts and scene pass, the particle pass, the
  3D view's projection, render-scale mode, sharpen setter, super-sampled
  draw, point projection and view copy; the level table and jitters read
  from the executable's data; the program's message loop.

## Out of scope

The OpenGL/Win32 glue (pixel formats, WGL context, window classes,
cursors, clipboard, message boxes, thread primitives), Steam Input, the
log file, drawing the status line and overlay, stereo rendering, the
jittered four-sub-view drawing itself (Exarust keeps its ordered 2 × 2
grid), the per-sub-view animation time, the decal bias and particle
pre-pass as render nodes, and the hook lists.

## Phase

Phase 8 (performance and settings: frame pacing, window and display
modes) and Phase 5 (visual fidelity: the renderer's pass order and the
3D view's rules).
