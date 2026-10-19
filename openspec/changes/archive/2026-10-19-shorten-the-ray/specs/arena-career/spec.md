## MODIFIED Requirements

### Requirement: Scene walk
When the scene has sectors, a sector that shares no camera bit SHALL be skipped. A body SHALL be visited only when its flags include 0x89, it shares a camera bit, and it lacks bit 0x4000000. A clear probe SHALL record the bits of 0.1, and any other probe the bits of 5. The ray SHALL start at 4000 along z. A centre past that length SHALL miss. A closer hit SHALL shorten the ray so a farther centre is left. A meeting hit SHALL replace the stored hit. An empty scene SHALL leave the supplied hit.

#### Scenario: The last positive body is the hit
- **WHEN** one sector shares no camera bit and another holds a skipped body, a probe of 1 with three hits, a body without 0x89, and a clear probe with two hits
- **THEN** the hit is the last body, the stored index is 1, and the pads are the bits of 5 then of 0.1

#### Scenario: A farther centre is left
- **WHEN** a centre at 100 is followed by a centre at 500, and another scene holds only a centre at 5000
- **THEN** the first hit stays at 100 with reach 100, and the centre at 5000 misses

### Requirement: Pane size
A new pane SHALL double the row one before the slot and cap it when greater. Bit 0x1000 SHALL choose the wide slot. Inner height SHALL be the unit times 6 plus the rows. Each other inner size SHALL be twice the margin plus that span word times the unit. Width SHALL be twice the pad plus the first inner size. Height SHALL be the unit times 2 plus the other sizes and the pad.

#### Scenario: Slot 1 sizes the pane
- **WHEN** a marked item builds with unit 2, margin 5, pad 6, span words 3 and 4, row 9, and a cap of 100
- **THEN** the rows are 18, the width is 28, the height is 58, and the item is stored

## ADDED Requirements

### Requirement: Child layout
A stretch child SHALL grow by the pane's change from its previous size. An anchor child SHALL move by that change. A centre child SHALL be placed at half the spare. Nested children SHALL not be visited.

#### Scenario: Three children take their places
- **WHEN** a pane of width 28 and height 58 lays out a centre child of 10 by 4, an anchor child at x 4, and a stretch child of width 10
- **THEN** the centre sits at 9, 27, the anchor moves to 32, and the stretch width is 38
