# Proposal

## Why

Phase 7 sizes a shell's index upload. The play draw does not submit that upload.

## What Changes

- A play draw of a skinned shell counts the room, then submits the upload when the flag is set.
- The mesh holds one index per triangle of the upload's payload. A buffer name above 0 unmaps.
- A clear flag keeps the counted room and does not unmap, even when a name is passed.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the play draw that submits a shell's index upload.

## Impact

- `exa_game::shell_draw`, calling the shipped index room and upload.
- `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the index upload and the finish that unmaps a named buffer. No new corpus files. Tests use synthetic spans.
- Out of scope: issuing the original's GL calls from the Bevy render graph.
