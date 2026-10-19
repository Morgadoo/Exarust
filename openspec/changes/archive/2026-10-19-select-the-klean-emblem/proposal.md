# Proposal

## Why

Phase 7 can name a company, but a typed name does not yet select an emblem. The name `klean` selects a known background, a known design, and four layer bytes.

## What Changes

- The name `klean` selects background 52, design 81, and the layer bytes 7, 0, 0, 6.
- Any other name selects nothing.
- Background 52 is named `emblembkg052`. Design 81 is named `emblemdes081`.

## Capabilities

### New Capabilities

- `company-emblem`: the preset a company name selects, and the picture that index names.

### Modified Capabilities

## Impact

- `exa_game::emblem`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, `crates/exa_game/README.md`.
- Evidence: the name compare, the four stored bytes, and the background and design picture names built from the index. No format field changes, so the corpus step is a skip.
- Out of scope: the other emblem controls, painting the picture, and a candidate's listed price. That price is a coin dword passed into the hire; the code that writes it was not read.
