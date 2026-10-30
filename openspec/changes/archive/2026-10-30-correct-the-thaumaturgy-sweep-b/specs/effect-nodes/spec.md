# Spec Delta

## ADDED Requirements

### Requirement: Rain pass matrices
Each rain pass SHALL upload as `ObjMat` the drop set's turn with every entry multiplied by the single-precision square root of the camera field times the single 1/70, placed at the drop box's centre and multiplied by the pass's sub-frame composite, and as `EyeVec` the camera less the box centre dotted with each scaled row.

#### Scenario: A field of 280
- **WHEN** the camera field is 280
- **THEN** the turn's entries are doubled and the object matrix's last row is the box centre with w = 1

### Requirement: Rain passes
The rain SHALL draw one pass per sub-frame with its turn two degrees about the first axis, followed, when the reflection level is above three, by a reflected pass at −2° about the first axis and 170° about the second.

#### Scenario: Reflection level three
- **WHEN** the reflection level is three
- **THEN** each sub-frame draws one pass

### Requirement: Vanish phase table
The displacement unit's phase table SHALL hold eight entries: (0, 0, −16, none), (0x1100, 0, 350, stage), (0x600, 350, 450, reveal), (0x300, 450, −16, show), (0, 500, −16, none), (0x200, 0, 200, stage), (0xe00, 200, 400, hide) and (0x1100, 400, −1, none).

#### Scenario: The table's ends
- **WHEN** the table is read
- **THEN** entries 0, 3 and 4 end at −16 and entry 7 at −1

### Requirement: Vanish phase advance
The frame callback SHALL run an entry's callback and advance the phase only when the elapsed time exceeds its end compared unsigned; the phased body's hook SHALL hide and let go on the −1 entry and otherwise move to phase 4 and show.

#### Scenario: The reveal
- **WHEN** the elapsed time passes 450 ms in phase 2
- **THEN** every body is shown 1 with the show call 0x20, the items are shown 1 and the phase becomes 3

#### Scenario: The last entry
- **WHEN** the phase is 7 at any elapsed time
- **THEN** the frame callback does not advance it

### Requirement: Force geometry pass uploads
A force geometry with more than one pass SHALL upload pass 0's object matrix, set pass 0's viewport and draw, then for each later pass set its viewport, upload its matrix and draw; with a single pass it SHALL draw nothing on that path.

#### Scenario: One pass
- **WHEN** the view has a single pass
- **THEN** no object matrix is uploaded by the force geometry's frame
