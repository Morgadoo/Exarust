## Why

The outdoor arenas showed a flat fog-coloured background where the original
draws a sky.

## What Changes

- Levels under the outdoor preset get an open sky: Bevy's physically based
  atmosphere (Hillaire 2020), lit by the level's own sun. It stands in for
  the original's sky shader, which is not decoded.
- Indoor levels keep their black background.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `level-lighting`: outdoor levels have a sky.

## Impact

- `exa_world::environment` and `docs/formats/RFC.md`.
