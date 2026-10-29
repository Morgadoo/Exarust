## ADDED Requirements

### Requirement: Base form key handler
With a focused record, the base form SHALL move the focus on Tab to the next enabled tab stop (the previous one with Shift, wrapping to the first or last tab stop), ending the key when there is none, and SHALL offer any other key to the focused record, which takes it when its answer has both low bits. A key nobody took SHALL go to the parent's key slot unless its flags have bit 1 (a repeat) or bit 2 (a typed character).

#### Scenario: Tab between records
- **WHEN** the first of three records is focused, the third is the next tab stop and Tab is pressed
- **THEN** the third record takes the focus

#### Scenario: A key nobody takes
- **WHEN** no record is focused and A is pressed
- **THEN** the key goes to the parent's key slot
