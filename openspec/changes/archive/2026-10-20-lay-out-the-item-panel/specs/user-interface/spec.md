## ADDED Requirements

### Requirement: Item-panel grid
The system SHALL shape the grid as `cols = round(√(n·W/H))`, `rows = ceil(n/cols)`, fill entries backwards from the last index, and spread the last partial row over the panel's width.

#### Scenario: Three entries
- **WHEN** three entries fill a 400×300 panel with a 160 cell
- **THEN** the grid is 2×2, the last entry is at (10, −15), the middle at (210, −15) and the first at (110, 135)

### Requirement: Item-panel relaxation
The system SHALL relax overlaps over five passes with divisors 1–5 for one entry and 4–8 for the grid, with slack 2 when the other entry is to the right and 1 otherwise, and SHALL scatter from the seed 0x80b71a with divisors 2, 2, 3, 3, 4.

#### Scenario: One entry against another
- **WHEN** the target is (150, 100) and the other (100, 100), both half 80, in a 400×400 panel
- **THEN** the target moves to (261, 261) and the other to (80, 80) after the clamp

### Requirement: Item-panel hit test and drop
The system SHALL hit-test from the last entry backwards over the fixed cell box, skipping `objflags & 0x100`; drop SHALL subtract the grab offset, clamp around the icon and raise the entry to the end; removal SHALL shift the tail left.

#### Scenario: Overlapping cells
- **WHEN** two cells are at (0, 0) and a point lies inside both
- **THEN** the hit is the higher index
- **WHEN** an entry at index 1 is dropped
- **THEN** it becomes the last entry

### Requirement: Item-panel accept and positions
The system SHALL reject a drag on the reject mask, the container's own id, a missing model, an oversized item, a capacity overflow, or a required-mask mismatch; `only_own` SHALL require a matching icon. Positions SHALL use sixteenths (lists) or eightieths (DB).

#### Scenario: Capacity and codec
- **WHEN** the capacity is 10, used 6 and the item bulk 5
- **THEN** the drag is rejected, unless it comes from the linked panel
- **WHEN** a position of 140 px is saved with unit 4
- **THEN** the list value is 560 and the DB value 2800
