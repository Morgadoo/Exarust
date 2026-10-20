## ADDED Requirements

### Requirement: Main-menu pages
The system SHALL provide the title, start, in-game, arena-pause, story-pause and class pages with their exact rows and order; CONTINUE SHALL be disabled without a save, VERSUS SHALL appear only when the arena is available, and NOVICE SHALL be gated.

#### Scenario: Title and start
- **WHEN** the title page is built without a save
- **THEN** its rows are BEGIN, CONTINUE, ARENA, MANUAL, SETTINGS, EXIT and CONTINUE is disabled
- **WHEN** the start page is built with the arena available
- **THEN** VERSUS appears before BACK and the page has five rows

### Requirement: Main-menu row origin
The system SHALL give the title page 0.38, the start page 0.45 or 0.42, the pause pages 0.39 and class select 0.45.

#### Scenario: Start page origins
- **WHEN** the start page has four rows
- **THEN** its origin is 0.45
- **WHEN** it has five
- **THEN** its origin is 0.42

### Requirement: Main-menu in-game row
The system SHALL label the in-game page's first row CONTINUE when a game is in progress and NEW GAME otherwise.

#### Scenario: In game and out
- **WHEN** a game is in progress
- **THEN** the first row is CONTINUE
- **WHEN** it is not
- **THEN** the first row is NEW GAME
