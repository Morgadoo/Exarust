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
- Evidence: the 102-byte `0x10012c850`: the `0x2c` copy, the stamp `0x1e0` plus 2 against the clock, the pending byte `0x1e4` and the callback slot `0x1b8`. The 113-byte `0x10012c8c0`: the body `0x160`, its `0x2e0` and `0x2d0`, the pane `0x180` targeting `0x168` and the link `0x170` targeting `0x160`. The 117-byte `0x10012c940`: the frame `0x158 + 0x40` plus one, the `0x100040180` call, the unit times 12 at `0x180 + 0x4c` and `0x170`, and the `0x4ef320` guard. No new corpus files.
- Out of scope: the callback, pane, link and tick bodies.

## Phase

Phase 5, character creation. The view's link, attach and size.
