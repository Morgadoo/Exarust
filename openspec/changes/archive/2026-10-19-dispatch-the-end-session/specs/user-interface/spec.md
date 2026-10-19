## ADDED Requirements

### Requirement: End-session dispatch
Page 2 SHALL call 0x10018bf80 for action 1 and show form 0x1001243c0 for action 2. Page 4 SHALL call 0x100124650 for action 1 on form page 4, otherwise 0x100124820, and call 0x100124820 for action 2. Page 8 SHALL, for action 1 on form page 2, call 0x100132250, 0x10017b480 and 0x10012c2c0 and show 0x100124800; otherwise it SHALL show 0x100123d30 with the last-round flag. Any other page SHALL call its registered handler when present. Every path SHALL clear the action and field 0x204 and return 0.

#### Scenario: The built-in pages
- **WHEN** the page is 2 and the action is 1
- **THEN** 0x10018bf80 is called
- **WHEN** the page is 4 and the action is 1 on form page 4
- **THEN** 0x100124650 is called
- **WHEN** the page is 8, the action is 1 and the form page is 2
- **THEN** the three calls and form 0x100124800 are dispatched

#### Scenario: The fallback
- **WHEN** the page is 3 and a handler exists
- **THEN** the handler is called
- **WHEN** the page is 3 and no handler exists
- **THEN** nothing is dispatched
- **AND** every path clears the action and field 0x204 and returns 0
