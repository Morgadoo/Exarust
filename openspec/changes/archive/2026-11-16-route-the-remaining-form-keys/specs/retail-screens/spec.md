## MODIFIED Requirements

### Requirement: Skill screen keys
While the skill ring is up, any key the window procedure sends (a key-down, a repeat or a typed character) SHALL close it through the open routine's toggle, except during the reveal's first 100 ms and while it hides; a page or the compact tree SHALL not close on a key.

#### Scenario: The opening key and a later key
- **WHEN** the screen opened at clock 0 and K is pressed at 50 ms, then Escape at 150 ms
- **THEN** the screen stays up after K and closes after Escape, with the player's controls back on

#### Scenario: A held key
- **WHEN** K goes down at 50 ms and repeats at 150 ms
- **THEN** the repeat closes the screen
