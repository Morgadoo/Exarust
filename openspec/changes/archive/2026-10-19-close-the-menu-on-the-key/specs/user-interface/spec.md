## ADDED Requirements

### Requirement: Menu close key
A key mask carrying both 0x2000 and 0x2 SHALL close the form with argument 0 and return 1 without running the base handler. Any other mask SHALL return the base handler's result and SHALL NOT close.

#### Scenario: The close mask
- **WHEN** the mask is 0x2002
- **THEN** the form closes and the result is 1
- **AND** the base handler does not run

#### Scenario: Any other mask
- **WHEN** the mask is 0x2000 or 0x2 and the base handler returns 7
- **THEN** the result is 7 and the form does not close
