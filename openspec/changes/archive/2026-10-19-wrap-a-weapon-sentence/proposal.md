# Proposal

## Why

A card can pick a description phrase. It does not yet wrap that phrase into the weapon sentence.

## What Changes

- A name of two letters or fewer stays as it is. A longer name drops a trailing dot.
- A bracket ending loses its last four letters. A `c` before the bracket keeps the name plus "It is" and a condition phrase.
- A leading dash is dropped and uses the high and great tables. Any other name uses the made table, then flawless, and perfect when flawless adds nothing.
- Quality and condition are that byte divided by 2.55, dropping the fraction.
- A bracketed name is the name, "It is ", the quality phrase, then ", but" when both marks are at most 50 or both are above, and " and" otherwise.
- An unbracketed name is the quality phrase, a space, and the name, then "An " or "A ".

## Capabilities

### New Capabilities

### Modified Capabilities

- `inventory-pane`: the sentence around a description phrase.

## Impact

- `exa_items::card`, calling the shipped phrase picker.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and the `exa_items` README.
- Evidence: the weapon sentence. No new corpus files. Tests use synthetic phrases.
- Out of scope: the fixed-id name edit, and the retail phrase tables.
