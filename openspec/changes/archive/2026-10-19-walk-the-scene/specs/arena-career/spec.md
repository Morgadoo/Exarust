## MODIFIED Requirements

### Requirement: Kind release
A cursor below 0, or at the view width or height, SHALL clear the hit and SHALL not count a kind call. A cursor inside SHALL keep a supplied hit. A marked item SHALL then release: a count above 0 drops and becomes 0, and a count of 0 or less clears a pointer, counting a close when the byte is 1. With no matching pane a marked span counts a build. The first pane tagged 0xce1c2013 for that item is shown, and a later pane is not. The ray distance and child layout are not part of this step.

#### Scenario: A cursor on the view edge does not release
- **WHEN** the cursor is at width 8 on a view of 8, the item is marked, and the held count is 2
- **THEN** the hit is clear, the kind call is not counted, and the count stays 2

#### Scenario: A marked item drops a count and shows the first pane
- **WHEN** a count of 3 is released, a pointer with byte 1 is released, and a view corner shows the second of two tagged panes
- **THEN** the count becomes 0 with one drop, the pointer is cleared with one close, and pane 11 is shown once

## ADDED Requirements

### Requirement: Scene walk
When the scene has sectors, a sector that shares no camera bit SHALL be skipped. A body SHALL be visited only when its flags include 0x89, it shares a camera bit, and it lacks bit 0x4000000. A clear probe SHALL record the bits of 0.1, and any other probe the bits of 5. A hit count above 0 SHALL replace the hit and store one less than that count. A later positive count SHALL replace an earlier one. An empty scene SHALL leave the supplied hit. The ray distance is not part of this step.

#### Scenario: The last positive body is the hit
- **WHEN** one sector shares no camera bit and another holds a skipped body, a probe of 1 with three hits, a body without 0x89, and a clear probe with two hits
- **THEN** the hit is the last body, the stored index is 1, and the pads are the bits of 5 then of 0.1

### Requirement: Pane size
A new pane SHALL double the row one before the slot and cap it when greater. Bit 0x1000 SHALL choose the wide slot. Inner height SHALL be the unit times 6 plus the rows. Each other inner size SHALL be twice the margin plus that span word times the unit. Width SHALL be twice the pad plus the first inner size. Height SHALL be the unit times 2 plus the other sizes and the pad. Child layout is not part of this step.

#### Scenario: Slot 1 sizes the pane
- **WHEN** a marked item builds with unit 2, margin 5, pad 6, span words 3 and 4, row 9, and a cap of 100
- **THEN** the rows are 18, the width is 28, the height is 58, and the item is stored

### Requirement: Pane place
The place SHALL sit half a width left of the cursor. Below 0 it SHALL become 0. Past the screen it SHALL become the screen width minus the pane. The other place SHALL be at least the unit. The order SHALL be the base plus 0x100. The item id SHALL be stored.

#### Scenario: The place is clamped to the screen
- **WHEN** the cursor is 100 on a screen of 200, and again 30 on a screen of 20, with the same pane width of 28
- **THEN** the first place is 86 and the second is -8
