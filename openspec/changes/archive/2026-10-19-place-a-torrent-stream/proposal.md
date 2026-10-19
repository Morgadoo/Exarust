# Proposal

## Why

Phase 7 steps a torrent's level and channels. The stream mesh and the push are not in.

## What Changes

- A span rounds to eight tenths, to the nearest even integer.
- A stream sits at 16 times its direction, then its height times 1.5 minus 30.
- The mesh asks for 13 groups for each stream plus 8, 12 bytes a group, and 8 indices a group.
- A reach is the radius times the scale, plus the margin, plus 10.
- A hit raises the channels, and a point inside the reach is pushed.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the torrent stream mesh and its push.

## Impact

- `exa_game::thaum`, beside the torrent mix.
- `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the stream birth, the mesh counts, and the push. No new corpus files. Tests use synthetic spans.
- Out of scope: the index winding, the shell mesh, and a character's impulse.
