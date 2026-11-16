## ADDED Requirements

### Requirement: Saved rows kept as captured
The bindings SHALL keep the saved table's rows as read or captured and save them unchanged, with the controller word the file was read with.

#### Scenario: Right control captured
- **WHEN** right control is captured for a row and the bindings are saved
- **THEN** the row is saved as the extended scan code 0x11D while the slot reads the left control

### Requirement: Key cells named from the rows
The Controls page SHALL name each key cell from its saved row, keeping the extended bit, with an empty caption for the primary and secondary mouse buttons.

#### Scenario: Left mouse row
- **WHEN** a row holds the primary mouse button
- **THEN** its cell's caption is empty
