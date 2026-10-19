## ADDED Requirements

### Requirement: Inventory close key
A key mask carrying both 0x2000 and 0x2 SHALL close with argument 0 and return 1. It SHALL use the stored callback when one exists and the form close otherwise. Any other mask SHALL return the base handler's result and SHALL NOT close.

#### Scenario: The callback path
- **WHEN** the mask is 0x2002 and a callback exists
- **THEN** the callback runs with argument 0, the form close does not run and the result is 1
- **WHEN** the mask is 0x2002 and no callback exists
- **THEN** the form close runs and the result is 1

#### Scenario: Any other mask
- **WHEN** the mask is 0x2000 and the base handler returns 7
- **THEN** the result is 7 and nothing closes
