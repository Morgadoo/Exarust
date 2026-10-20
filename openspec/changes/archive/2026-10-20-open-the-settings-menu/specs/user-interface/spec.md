## ADDED Requirements

### Requirement: Settings menu lifecycle
Opening the settings menu SHALL create it when none exists and otherwise re-show it, SHALL lay it out, SHALL select entry 1 behind the resume flag, and SHALL finish with −1. Closing SHALL act only on an existing menu, and clearing its flag SHALL act only on an existing menu.

#### Scenario: Created, shown and absent
- **WHEN** no settings menu exists and the resume flag is clear
- **THEN** the opening creates it, lays it out, selects nothing and finishes with −1
- **WHEN** a settings menu exists and the resume flag is set
- **THEN** the opening re-shows it and selects entry 1
- **WHEN** no settings menu exists
- **THEN** closing and clearing its flag do nothing
- **WHEN** one exists
- **THEN** closing and clearing its flag each act once
