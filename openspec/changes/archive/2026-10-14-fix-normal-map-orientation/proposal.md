## Why

Bumps on walls and floors looked inverted. The original's bitangent runs
along −v of the stored UVs. Bevy's tangent generation flips the MikkTSpace
sign, so its bitangent already runs along −v too. The shader still negated
the normal map's green channel, which flipped every bump a second time.

## What Changes

- Use the normal map's channels as stored.
- Correct the tangent-frame notes in `docs/RENDERING.md`.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `rendering`: the normal-map orientation.

## Impact

- `exa_render` (`exa.wesl`) and `docs/RENDERING.md`.
