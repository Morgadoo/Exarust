## MODIFIED Requirements

### Requirement: Shown tree takes the keys
While the power tree is shown in play, every key the window procedure sends (a key-down, a repeat, a typed character, with its word) SHALL go to the tree's key rule: the bound next-power and previous-power keys, not as a typed character, SHALL step the bar's chosen slot, and any other key SHALL close the tree through its open-or-close routine, except while the opening fade's stamp less 950 ms is still ahead of the clock and while the closing fade runs.

#### Scenario: Any key closes the tree
- **WHEN** the tree has been shown for a second and W is pressed
- **THEN** the tree fades out over 256 ms and its canvas hides

#### Scenario: The opening guard
- **WHEN** a key is pressed 40 ms after the tree was shown
- **THEN** the tree stays open

#### Scenario: A rebound power key
- **WHEN** next-power is bound to E and E goes down on the shown tree
- **THEN** the bar steps and the tree stays open
