# Proposal

## Why

Hovering a skill stores the index, validates it against the screen's grid byte, keeps the previous and last positive indices and rate-limits a 200-unit stamp. Exarust has the position, show, leave, ring and hit test but not the hover.

## What Changes

- A positive index or current value stores the index and returns 3.
- The candidate is the index when bits 8–10 and the low 3 bits are positive and its grid byte is set, else 0.
- An unchanged candidate leaves the stamps; a new positive candidate with a different last positive stamps the clock.
- Otherwise the stamp becomes the elapsed with the under-200 correction.
- The candidate becomes previous and, when positive, last positive.
- With both values non-positive the base runs.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skill screen's hover.

## Impact

- `exa_ui::skill`, from `skill_hover`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `TSkillUI` moves from 7% to 9%.
- Evidence: the 287-byte hover: the early base call, the index field, the grid and its `row × 8 + column + 1` byte, the previous and last positive indices, the stamp, the 200-unit limit and the game clock. No new corpus files.
- Out of scope: the base method.

## Phase

Phase 4, UI. The skill screen's hover.
