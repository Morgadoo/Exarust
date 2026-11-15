## ADDED Requirements

### Requirement: Older grid layout word
The grid's older-layout word (the header's third word) SHALL be 0 in every shipped grid and in every grid the export writes, so the older plane conversion does not run on 0.9.5's data.

#### Scenario: The installed game's grids
- **WHEN** every grid chunk of the installed packs is read
- **THEN** each has 0 and 600 in its third and fourth header words
