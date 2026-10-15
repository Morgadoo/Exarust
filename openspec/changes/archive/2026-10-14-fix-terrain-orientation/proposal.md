## Why

Next to the original at the same camera, exanima01's terrain showed its
cobblestones laid out differently. The texture was right but mirrored: v
ran along +Z, whereas the original's terrain, like its floors, runs v along
−Z.

## What Changes

- Terrain texture coordinates: u along +X, v along −Z.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `terrain`: texture orientation.

## Impact

- `exa_world` (`terrain.wesl`).
