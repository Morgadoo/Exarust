# Proposal

## Why

A weapon sentence is built when the id word is not -1. The name edit used when that word is -1 is not in yet.

## What Changes

- A name shorter than three letters stays unchanged, and the roll does not step.
- A longer name deletes a 0x10 byte and the byte after it, shortening by two.
- "An " or "A " is added only when the first byte is at least Z.

## Capabilities

### New Capabilities

### Modified Capabilities

- `inventory-pane`: the fixed-id name edit.

## Impact

- `exa_items::card`, on the weapon sentence.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and the `exa_items` README.
- Evidence: the fixed-id branch of the weapon sentence. No new corpus files. Tests use synthetic bytes.
- Out of scope: the apparel sentence.
