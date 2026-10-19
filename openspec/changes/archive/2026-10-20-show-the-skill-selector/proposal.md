# Proposal

## Why

The skill selector, when shown, clears bits 0x60 on its object and applies the positive selection, or −1 for words nine and ten. Exarust does not model that show.

## What Changes

- With no object nothing is applied and the base still runs.
- With an object, bits 0x60 clear.
- A positive selection applies itself; otherwise words nine or ten apply −1.
- Any other word applies nothing.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skill selector's show.

## Impact

- `exa_ui::menu`, from `show_skill_selector`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 141-byte selector show's bit clear and two word guards. No new corpus files.
- Out of scope: the apply call and the base.

## Phase

Phase 7, the interface. The show path.
