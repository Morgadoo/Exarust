# Proposal

## Why

The skill screen lays its nodes around a ring: the slot picks an angle from the node count, the turn rotates it, and the two offsets and the unit give the radius. Exarust has the screen's show, leave and index placement but not the ring.

## What Changes

- The angle is `(slot − 3.5) × 2π / count + 1 − turn`.
- The radius is the product of the two offsets, the unit and the cosine (x) or sine (y), rounded to the nearest even integer and multiplied by the turn.
- The centre x is half the width, less one unit while scrolled with a positive page.
- The centre y is half the height, less three units on page 0.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skill ring's node position.

## Impact

- `exa_ui::skill`, from `skill_ring_position`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `TSkillUI` moves from 2% to 4%.
- Evidence: the 333-byte ring placement: the `3.5`, `2π`, `1.0`, the turn field, the node count, the two offsets, the size fields, the page, the scrolled byte and the unit global. The cosine/sine order is fixed by both ring routines using the first output for x. No new corpus files.
- Out of scope: the engine's sine/cosine routine, already used as a primitive.

## Phase

Phase 4, UI. The skill ring's node position.
