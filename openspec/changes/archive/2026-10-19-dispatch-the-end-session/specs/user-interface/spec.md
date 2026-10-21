## ADDED Requirements

### Requirement: End-session dispatch
Page 2 SHALL continue for action 1 and show the main menu for action 2. Page 4 SHALL reload the stored level for action 1 on form page 4, otherwise end the session to the main menu, and end it to the main menu for action 2. Page 8 SHALL, for action 1 on form page 2, hide the kept form, make its second call, prepare the character record and show its form; otherwise it SHALL show the next practice fight with the last-round flag. Any other page SHALL call its registered handler when present. Every path SHALL clear the action and the hovered selection and return 0.

#### Scenario: The built-in pages
- **WHEN** the page is 2 and the action is 1
- **THEN** the continue runs
- **WHEN** the page is 4 and the action is 1 on form page 4
- **THEN** the stored level is reloaded
- **WHEN** the page is 8, the action is 1 and the form page is 2
- **THEN** the three calls and page 8's form are dispatched

#### Scenario: The fallback
- **WHEN** the page is 3 and a handler exists
- **THEN** the handler is called
- **WHEN** the page is 3 and no handler exists
- **THEN** nothing is dispatched
- **AND** every path clears the action and field 0x204 and returns 0
