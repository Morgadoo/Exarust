## MODIFIED Requirements

### Requirement: Manual input
While the manual is open, a list click or the arrow keys SHALL pick a topic, and F1 or a right click SHALL close it and uncover the menu; Escape SHALL climb to the root task's menu toggle, which closes the manual with the open menu in a session and leaves it up in the menu's phase; the menu SHALL take no input until then.

#### Scenario: Closing with F1
- **WHEN** F1 is pressed with the manual open over the menu
- **THEN** the manual closes and the menu stays open

#### Scenario: Escape in a session
- **WHEN** the manual is open over the in-game menu and Escape goes down
- **THEN** the menu and the manual close

