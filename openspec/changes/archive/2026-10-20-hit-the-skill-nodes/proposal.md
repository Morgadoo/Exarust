# Proposal

## Why

The skill screen hit-tests each ring node by scanning five satellite points around its centre, folding the sample index into a result. Exarust has the node positions but not the hit test.

## What Changes

- A node qualifies while the mouse is within 23 units of its centre.
- Five samples sit at 18.3 units around the node, 0.2 of a turn apart, with `x + round(sin) × turn` and `y − round(cos) × turn`.
- A sample within 5 units ORs its index into the result and adds one.
- The result starts at 0 on page 2 and −1 otherwise.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skill ring's node hit test.

## Impact

- `exa_ui::skill`, from `skill_node_hit`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `TSkillUI` moves from 4% to 7%.
- Evidence: the 512-byte `0x100224f30`: the page check at `0x1c0`, the count at `0x1a8`, the outer 23 units, the four-point loop with `0.2` at `0x10033acb8` and `18.3` at `0x10033acc0`, the turn at `0x178`, the inner 5 units, and the `(index | result) + 1` fold. No new corpus files.
- Out of scope: the node-position virtual at slot `0x1d0`, supplied as a list.

## Phase

Phase 4, UI. The skill ring's node hit test.
