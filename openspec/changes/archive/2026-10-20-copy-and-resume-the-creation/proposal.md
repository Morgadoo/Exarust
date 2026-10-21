# Proposal

## Why

The creation screen copies a three-word point from its child and resumes by saving, refreshing at the current index and calling the resume slot. Exarust has the screen start and destroy but not the copy or the resume.

## What Changes

- The copy takes the child's three words into the object's point.
- Resuming saves the global list, refreshes at the current index and calls the resume slot.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the point copy and resume.

## Impact

- `exa_game::create`, from `copy_creation_point` and `resume_creation`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` stays about 32% (about 32.5%).
- Evidence: the 39-byte copy from the child's record into the screen. The 71-byte resume: the save call, the refresh with the child's two fields and the slot callback. No new corpus files.
- Out of scope: the save, refresh and slot bodies.

## Phase

Phase 5, character creation. The point copy and resume.
