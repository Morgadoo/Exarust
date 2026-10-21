# Proposal

## Why

The creation screen also rebuilds a second option list, or only stores its index when the record's band is non-zero. Exarust has the first list rebuild but not the second.

## What Changes

- A non-zero band stores the index at `0x1a4` and the child's `0xa4` and stops.
- Otherwise the list is built and a non-positive count fails.
- On success the index is stored, the old list is replaced, the sub-routine runs, widget 9 is captioned with `index + 0x41` and the list is drawn when allowed.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the second list rebuild.

## Impact

- `exa_game::create`, from `rebuild_creation_second_list`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 16% to 17%.
- Evidence: the 414-byte second-list rebuild: the `0xff00` band test on the child's appearance word, the stored index and child, the list constructor, the list, the sub-routine, widget 9's `0x41` caption, the busy guard and the draw. No new corpus files.
- Out of scope: the constructor, sub-routine and draw.

## Phase

Phase 5, character creation. The second list rebuild.
