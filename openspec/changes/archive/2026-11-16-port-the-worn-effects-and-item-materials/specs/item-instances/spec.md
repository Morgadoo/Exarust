## ADDED Requirements

### Requirement: Copy bulk from the template
The bulk of a tagged item SHALL be the model section's bulk byte of the record its header names as template, one step, and 0 when that record or its model section is missing; the copy's own byte SHALL NOT count.

#### Scenario: A copy with its own byte
- **WHEN** a level copy with bulk byte 9 names a template with bulk 3
- **THEN** its bulk is 3, and a copy naming no template weighs 0
