# Proposal

## Why

An item card can show a generated description phrase. The picker that chooses that phrase is not in yet.

## What Changes

- A phrase is a candidate when its low and high include the value, in table order.
- No candidate leaves the roll unchanged and adds nothing.
- Otherwise the roll steps by multiplying by 0x08088405 and adding 1, and the high half of that times the candidate count chooses one.
- A candidate that repeats a word longer than three letters from the text so far adds nothing. Each string's last character is ignored, and letter case does not count.
- A chosen phrase that starts with a comma is that phrase plus a dot. Any other chosen phrase is a space, the phrase, and a dot.

## Capabilities

### New Capabilities

### Modified Capabilities

- `inventory-pane`: how a description phrase is chosen.

## Impact

- `exa_items::card`, beside the rank gate.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and the `exa_items` README.
- Evidence: the phrase picker. No new corpus files. Tests use synthetic text.
- Out of scope: the sentence built around the phrase, and the retail phrase tables.
