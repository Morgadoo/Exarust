## ADDED Requirements

### Requirement: Takes carried out
A take SHALL despawn a loose item, remove a container's first matching entry, release an item from the corpse's grip (the hand and its carried entry) or else remove it from the worn list, or remove a carried entry. Wearing SHALL put the item on and out of the carried list before taking it from its source.

#### Scenario: Each source
- **WHEN** a container, a corpse wearing one item and gripping a carried one, and a loose item are taken from
- **THEN** each take edits its source, and the gripped item leaves the hand and the carried list but not the worn list

### Requirement: Drops carried out
Dropping SHALL wait while the pack is locked, need the item in the pack, and refuse an item whose size is unknown or over the container's largest item; otherwise the item SHALL go into the container and out of the pack.

#### Scenario: Sizes against the largest item
- **WHEN** items of sizes 4 and 5 and one of no size are dropped into a container taking items up to 4, first with the pack locked
- **THEN** the locked drop waits, the size-4 item goes in, and the others stay out
