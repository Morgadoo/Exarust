## ADDED Requirements

### Requirement: Line opening wrapper
Opening the line SHALL call the parent's open once and the texture load once.

#### Scenario: The wrapper makes both calls
- **WHEN** the line is opened
- **THEN** the parent call count is 1 and the texture call count is 1
