# Proposal

## Why

Phase 7 steps a blast's chunks, but a barrier's ripple and the dome extent are not stepped.

## What Changes

- A ripple loses 0.05 of strength each step. The shown value moves toward that strength by at most 0.04.
- Both under 0.07 removes the ripple. While it remains, its place eases 0.05 of the way toward the hit.
- The dome extent is 30 plus 1.25 times the ripple count.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the barrier ripple and the dome extent.

## Impact

- `exa_game::thaum`, beside the blast charge.
- `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the dome ripple step and the stored extent of 30. No new corpus files. Tests use synthetic ripples.
- Out of scope: the polyhedron mesh, wave, torrent and shell.
