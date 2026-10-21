# Proposal

## Why

Levels are lit through the original's scene environment plug-in: the sun
and moon on a day path, a daylight factor, a track of six day records,
cloud dimming, fog narrowing, the ambient and clear colours, the automatic
sunlight rule, rain and lightning. Exarust had only the level presets and a
fixed exponential fog. The tile-grid builder's unit routines (special tile
masks per rotation, the `Terrain` floor set, the room lookups of nodes and
characters) were partly reproduced.

## What Changes

- `exa_world::scene_env`: a pure model of the plug-in (constructor
  defaults, setters, sun and moon placement with the engine's arc sine and
  approximate renormalisation, record pick and blend, cloud dimming, sky
  cube tint, fog span and per-frame near/far, clear colour and horizon
  colour, the sun mode, sky switch and automatic sunlight, rain fading,
  lightning with the engine's generator and the run-time library's,
  time of day), following the instructions' double-precision arithmetic
  and NaN behaviour.
- `exa_world::environment` builds each level's `Lighting` through the model;
  the outdoor preset's fog becomes linear between the model's near and far,
  as the original's shader fog is. Story and record-lit levels keep their
  lighting unchanged.
- `exa_world::grid`: special tile masks turn with the cell's rotation; a
  cell past the last piece of its type places the last, one with none the
  set's first; only a floor set named exactly `Terrain` is skipped.
- `exa_world::rooms`: nodes and characters find their room by the cell
  they stand in (a floor, clamped for nodes), not the nearest.

## Capabilities

### Modified Capabilities

- `level-lighting`: the scene environment model and linear outdoor fog.
- `grid-building`: rotated special masks, the nth special piece, the
  `Terrain` floor set, node room lookups.

## Impact

- `crates/exa_world` (`scene_env`, `environment`, `grid`, `rooms`,
  `room_view`), `docs/RENDERING.md` (Environment and weather),
  `docs/formats/RFC.md` (Environment, Grid building), the crate README.
- exanima01's navigation pins move: its special tiles now mask edges in
  every rotation, as the original's start-up table does.

## Evidence

Reverse engineering of `Exanima.exe` 0.9.5 through REA (Ghidra): the
plug-in's constructor, setters, sun placement, record blend and pick, sky
dimming, per-frame apply, rain request, strike setup, weather tick, sun
mode, sky switch, sunlight creation and automatic rule; the engine's arc
sine, normalisation, sRGB, colour-to-byte and length helpers; the grid
unit's mask rotation, start-up table, set loading, special-piece walk and
room lookups. Arithmetic was read from the instructions, not the
pseudo-code. Real-data checks: the navigation and lighting tests on the
installed game.

## Out of scope

- The day record table (a constant table in the executable, set up by the
  environment unit) and so the record-driven sky, sun and fog colours.
- Driving the weather and time of day in play, the rain and thunder sound
  and effect objects, the lightning's camera-relative placement.
- Uploading the tinted sky cube, the sky object's node updates, the sun
  light's cloud dimmer on Bevy's light.
- The engine's fast sine, logarithm and exponent (the library functions
  stand in), cross-room light lists, the sector node export, the per-cell
  floor merge.

## Phase

Phase 2 (static world: level lighting) and Phase 5 (visual fidelity:
environment and fog).
