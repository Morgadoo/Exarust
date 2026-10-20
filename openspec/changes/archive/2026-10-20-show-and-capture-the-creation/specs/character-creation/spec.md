## ADDED Requirements

### Requirement: Creation form close
Closing the form SHALL clear the stored byte and schedule the special form in mode 10, otherwise the regular form.

#### Scenario: Special and regular
- **WHEN** the mode is 10
- **THEN** the stored byte clears and the special form is scheduled
- **WHEN** the mode is 9
- **THEN** the regular form is scheduled

### Requirement: Creation key
A key with a selection SHALL forward and return the base result. Without one it SHALL return `0x40` and close.

#### Scenario: With and without a selection
- **WHEN** a selection exists and the base returns 3
- **THEN** the key returns 3 and does not close
- **WHEN** no selection exists
- **THEN** the key returns `0x40` and closes

### Requirement: Creation show
Showing SHALL prepare, dispatch, refresh and build when the check passes.

#### Scenario: Build and skip
- **WHEN** the check passes
- **THEN** the prepare, dispatch, refresh and build all run
- **WHEN** the check fails
- **THEN** the prepare, dispatch and refresh run and the build does not

### Requirement: Creation capture
Capturing SHALL copy six words, compute `current − last + saved`, store the mode, clear the stored byte and schedule.

#### Scenario: The capture
- **WHEN** the current is 10, the last 4 and the saved 7
- **THEN** the value is 13
- **AND** the six words, the mode and the scheduled flag are stored
