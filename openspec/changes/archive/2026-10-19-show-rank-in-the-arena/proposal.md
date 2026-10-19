# Proposal

## Why

A shield card can draw a rank and a points row, but those rows stay off until a mode turns them on.

## What Changes

- Entering arena turns rank and points on. Entering story turns them off. Entering practice turns them off unless the mode being left already has bit 2. The same mode, and any other mode, leaves them.
- A shown rank uses the skill byte's top three bits. Zero is absent. The names are Aspirant, Novice, Adept, Expert, Master, and a dash. The seventh index has no name.
- Shown points are the low five bits times one half. Zero is absent.

## Capabilities

### New Capabilities

### Modified Capabilities

- `inventory-pane`: when a card shows rank and points.

## Impact

- `exa_items::card`, beside the shield rows.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and the `exa_items` README.
- Evidence: the mode switch that writes the two enable bytes, and the rank table. No new corpus files. Tests use synthetic numbers.
- Out of scope: description phrases.
