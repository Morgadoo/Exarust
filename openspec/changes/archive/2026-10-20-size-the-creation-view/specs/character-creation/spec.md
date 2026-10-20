## ADDED Requirements

### Requirement: Creation link copy
Copying SHALL copy the link word, then compare the stamp plus 2 with the clock. At or below it the stamp SHALL take the clock, the pending flag SHALL clear and the callback SHALL run; above it the pending flag SHALL set.

#### Scenario: Pending and firing
- **WHEN** the stamp is 10 and the clock 11
- **THEN** the pending flag sets and the callback does not run
- **WHEN** the stamp is 10 and the clock 12
- **THEN** the stamp becomes 12, the pending flag clears and the callback runs

### Requirement: Creation body attach
Attaching SHALL store the body, its camera and its link, give a present pane the body's `0x1f0` and a present link the renderer's `0x30`.

#### Scenario: Pane and link
- **WHEN** the pane and link are present
- **THEN** the body, camera and link are stored and both targets are written
- **WHEN** the pane is absent
- **THEN** its target is not written

### Requirement: Creation view size
Sizing SHALL take the frame plus one, or one with no frame, store the unit times 12 and tick when the global is present.

#### Scenario: With and without a frame
- **WHEN** the frame is 3 and the unit 4
- **THEN** the count is 4 and the size is 48
- **WHEN** there is no frame
- **THEN** the count is 1
- **WHEN** the global is present
- **THEN** the tick runs
