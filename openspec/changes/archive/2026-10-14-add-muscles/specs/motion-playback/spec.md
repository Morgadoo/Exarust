## ADDED Requirements

### Requirement: Clips in place
The system SHALL remove a clip's horizontal root travel (the difference between its last and first keys, spread over the clip) when posing a body, since the character controller moves the body.

#### Scenario: Walk cycle
- **WHEN** the walk loops
- **THEN** the pelvis stays over the character instead of advancing and snapping back
