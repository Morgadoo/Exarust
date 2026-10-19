# Proposal

## Why

A weapon sentence is in. The apparel card builds a different sentence around the same phrase, and that sentence is not in yet.

## What Changes

- A bracket drops four letters. `p` without `c` uses "They are ". `c` adds "It is" and skips quality. Any other bracket uses "It is ".
- The same side of 50 joins with ", but", and differing sides with " and".
- A leading "A " is stripped. A dot that is not last keeps a tail after the condition phrase.
- Quality reads the first 14 made rows, then flawless, then perfect.
- Material 1 through 8 uppercases a side prefix. A 0x10 with that material is replaced by the name and a space. Any other 0x10 is deleted.
- An id of -1 uses the fixed-name edit and does not step the roll.

## Capabilities

### New Capabilities

### Modified Capabilities

- `inventory-pane`: the apparel sentence.

## Impact

- `exa_items::card`, beside the weapon sentence.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and the `exa_items` README.
- Evidence: the apparel sentence builder. No new corpus files. Tests use synthetic bytes.
- Out of scope: retail phrase tables, and the original GL calls for a shell upload.
