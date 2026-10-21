# Proposal

## Why

The creation view copies a link word, attaches a body with its camera and link, and sizes itself from the current frame. Exarust has the tick and record but not these view routines.

## What Changes

- The link word is copied, then a two-unit debounce either fires the callback or sets the pending flag.
- Attaching stores the body, its camera and its link, and targets the pane and link when present.
- Sizing takes the frame plus one, or one with no frame, stores the unit times 12 and ticks when the global is present.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the view's link, attach and size.

## Impact

- `exa_game::create`, from `copy_creation_link`, `attach_creation_body` and `resize_creation_view`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 19% to 20%.
- Evidence: the 102-byte resize: the width copy, the stamp plus 2 against the clock, the pending byte and the callback slot. The 113-byte bind: the body, its two fields, the pane targeting the model and the link targeting the body. The 117-byte size: the frame plus one, the frame call, the unit times 12 stored twice, and the global guard. No new corpus files.
- Out of scope: the callback, pane, link and tick bodies.

## Phase

Phase 5, character creation. The view's link, attach and size.
