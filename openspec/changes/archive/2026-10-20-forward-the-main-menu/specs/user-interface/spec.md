## ADDED Requirements

### Requirement: Main menu forward
The main menu's forward action SHALL run action 0 on page 1 and action 1 on page 8, each entering the session, and SHALL sound only when the input is ready. Any other page SHALL do nothing.

#### Scenario: Page one, page eight and another page
- **WHEN** the page is 1 and the input is ready
- **THEN** the action is 0, the sound plays and the session entry follows
- **WHEN** the page is 8 and the input is not ready
- **THEN** the action is 1, no sound plays and the session entry follows
- **WHEN** the page is 2
- **THEN** no action, no sound and no session entry
