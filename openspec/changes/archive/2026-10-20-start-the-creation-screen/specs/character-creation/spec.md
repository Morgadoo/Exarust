## ADDED Requirements

### Requirement: Creation screen start
Starting the screen SHALL reset, reset again when the condition holds, create the form when absent and prepare it. In mode 12 it SHALL set the form mode to 2 with the arena flag and 1 otherwise, refresh, build when checked and tick, then store the form and the value 2.

#### Scenario: Arena, story and other
- **WHEN** the mode is 12 and the arena flag is set
- **THEN** the form mode is 2, the refresh runs, the build runs and the tick runs
- **AND** the form and 2 are stored
- **WHEN** the mode is 12 without the arena flag
- **THEN** the form mode is 1
- **WHEN** the mode is not 12
- **THEN** the refresh, build and tick do not run and the form is still stored

### Requirement: Creation apply step
The step SHALL relink, refresh the camera and run both ticks. It SHALL store the part result only when a target exists.

#### Scenario: With and without a target
- **WHEN** a target exists and the part is found
- **THEN** the link, camera, store, part and both ticks all run
- **WHEN** the target is absent
- **THEN** the store is skipped and the rest run
