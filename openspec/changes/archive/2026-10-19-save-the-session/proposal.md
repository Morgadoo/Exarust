# Proposal

## Why

Phase 7 asks for a save in an Exarust format. The original's packages are not a compatibility goal, and nothing round-trips a session.

## What Changes

- A session file starts with `EXRS` and version 1.
- It stores the mode, the story level, the journal, the chosen topics, the company and its roster, and the focus.
- A short file, a bad field, or a file that starts with the original save magic is refused.

## Capabilities

### New Capabilities

- `session-save`: writing and reading an Exarust session.

### Modified Capabilities

## Impact

- `exa_game::session`.
- `docs/PLAN.md`.
- Evidence: the session fields this tree already runs. The original package layout is the format this deliberately does not read. No retail format field changes, so the corpus step is a skip.
- Out of scope: the original's save packages, checkpoints, and the menus that pick a slot.
