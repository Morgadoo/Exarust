# Proposal

## Why

The skill screen's link search resolves the node under a point, tests the two satellite slots beside it, then scans five satellites around every node, ranking by a cone-adjusted distance and returning the chosen slot code with its point. Exarust has the point hit and satellite placement but not the search.

## What Changes

- The two slots beside the hit node are tested first: a reachable slot computes its satellite at 18.3 units and 0.2 of a turn, scales the y offset by 4 when the direction is mostly horizontal, normalizes it, and keeps the largest dot above 0.1.
- Without a kept slot, every node's five satellites are scanned; each needs a squared offset of at least 1 and a dot of at least 0.3, and the smallest length over the dot squared from a one-million start wins.
- The chosen code is returned with the satellite written to the point.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skill link search.

## Impact

- `exa_ui::skill`, from `pick_skill_satellite`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `TSkillUI` moves from 25% to 32%.
- Evidence: the 1363-byte link search: the direction, the axis flag, the node hit and point, the link table, the two-slot loop, the five-slot scan, the satellite constants 18.3 and 0.2, the 0.1 dot, the 1.0 squared-length floor, the 0.3 dot, the one-million start, `length / dot²`, and the y scaling by 4. No new corpus files.
- Out of scope: the hit, reach and layout bodies.

## Phase

Phase 4, UI. The skill link search.
