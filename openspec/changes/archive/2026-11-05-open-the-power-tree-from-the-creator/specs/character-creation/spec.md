## MODIFIED Requirements

### Requirement: Creation commit
On the equipment page, a record with known powers SHALL open the power tree. Otherwise:
- recruiting SHALL hand the record to the arena;
- versus SHALL hand the record to its lobby form, and with no lobby form READY SHALL do nothing;
- any other mode SHALL save the preset, commit the record, store its worth and start the game.

The appearance page's back action SHALL leave for the arena when recruiting. Otherwise it SHALL leave for the menu through the creator's cancel.

#### Scenario: Commit
- **WHEN** a record with two known powers is committed
- **THEN** the power tree opens
- **WHEN** the appearance page goes back in story
- **THEN** the screen leaves for the menu

#### Scenario: READY without a lobby
- **WHEN** a named versus character's READY is pressed with no lobby form
- **THEN** the creator stays on the equipment page and no session starts

#### Scenario: The unknown background's character
- **WHEN** the story creator opens for the unknown background
- **THEN** its record knows the two starting powers
- **AND** the equipment page's final button keeps the label it had and opens the power tree

## ADDED Requirements

### Requirement: Creation power tree
Opening the power tree from the creator SHALL hide the creator's form, set its page state to 2 and show the tree in its creation mode on the creator's record, with no calling form and no power bar.

#### Scenario: Opening
- **WHEN** a named unknown-background character presses the equipment page's final button
- **THEN** the creator's form hides and the tree shows its creation prompt, without the power bar

### Requirement: Creation power tree BEGIN
The tree's BEGIN (or the form's confirm key) SHALL close the tree and commit the record with the powers the tree wrote, starting the story in story mode and creating the character in every other mode.

#### Scenario: BEGIN
- **WHEN** the tree opened from the creator writes a new power and its BEGIN is pressed
- **THEN** the creator commits a record that knows the two starting powers and the new one, and the story starts

### Requirement: Creation power tree BACK
The tree's BACK (or the form's cancel key) SHALL close the tree and show the creator's form again with the page state left at 2: the final button then rebuilds the equipment page, and the creator's back action leaves the creator.

#### Scenario: BACK
- **WHEN** the tree's BACK is pressed
- **THEN** the creator's form shows again, its final button rebuilds the equipment page and a second press opens the tree again
- **AND** the creator's back action before that second press leaves for the menu
