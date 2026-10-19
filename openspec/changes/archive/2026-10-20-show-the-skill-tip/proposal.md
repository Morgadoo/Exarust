# Proposal

## Why

The skill tooltip searches the skill list for its marker row and, when found, stores the skill index and threshold, marks itself, places itself from the row anchor and stamps the clock. Exarust does not model it.

## What Changes

- A missing row shows nothing.
- A found row sets the index to object+0xb8 − 1 and the threshold to row+0x38 − 1.
- Flags gain 0x10; the place is the anchor's x and nine units plus its y.
- The height is 24 units when global 0x2ad3c0 is 0 and 18 units otherwise.
- The visible byte is set, three calls run, and the stamp is three times the base plus the stored offset.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skill tooltip's show.

## Impact

- `exa_ui::menu`, from `show_skill_tip`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 372-byte tooltip method's row scan, placement and stamp. No new corpus files.
- Out of scope: the row scan and the three calls.

## Phase

Phase 7, the interface. The tooltip placement.
