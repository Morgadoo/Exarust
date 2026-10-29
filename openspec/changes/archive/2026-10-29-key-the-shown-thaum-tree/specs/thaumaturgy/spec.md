## ADDED Requirements

### Requirement: Shown tree takes the keys
While the power tree is shown in play, every pressed key SHALL go to the tree's key rule: the right and left arrow keys SHALL step the bar's chosen slot, and any other key SHALL close the tree through its open-or-close routine, except while the opening fade's stamp less 950 ms is still ahead of the clock and while the closing fade runs.

#### Scenario: Any key closes the tree
- **WHEN** the tree has been shown for a second and W is pressed
- **THEN** the tree fades out over 256 ms and its canvas hides

#### Scenario: The opening guard
- **WHEN** a key is pressed 40 ms after the tree was shown
- **THEN** the tree stays open
