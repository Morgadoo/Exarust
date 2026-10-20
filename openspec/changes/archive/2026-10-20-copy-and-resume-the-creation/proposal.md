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
- Evidence: the 39-byte `0x10012f590`: the copy from `0x158 + 0xc` into `0x10`. The 71-byte `0x10012f600`: the save `0x10017f500`, the refresh `0x10012f950` at `0x254` and the slot `0xc8`. No new corpus files.
- Out of scope: the save, refresh and slot bodies.

## Phase

Phase 5, character creation. The point copy and resume.
