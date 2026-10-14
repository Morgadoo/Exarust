# Proposal

## Why

Story levels place their inhabitants (villagers, zombies, corpses) with
their own records and worn items. None appeared.

## What Changes

- `exa_formats::characters`: placements (position, facing, condition, pose)
  and the level's character records up to their worn items. `Scene` gains
  them and the level's local object database.
- `exa_world`: placed characters resolved to models and appearances (local
  or global items).
- `exa_character::Outfit` carries each garment's appearance.
- Play mode spawns placed humans dressed in their items. The dead lie down.
- `exarust --at x,y,z` starts play mode elsewhere. `exa-tool corpus levels`
  counts placed characters and resolved items.

Out of scope: other races' bodies, roles and AI, held weapons, exact
corpse poses.

## Capabilities

### New Capabilities

### Modified Capabilities
- `level-decoding`: characters and local items.

## Impact

`crates/exa_formats`, `crates/exa_world`, `crates/exa_character`,
`crates/exa_tool`, `crates/exarust`, docs.
