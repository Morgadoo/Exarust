## ADDED Requirements

### Requirement: Creator reached from the menu
The main menu's BEGIN SHALL open the story background picker, and confirming a card SHALL open the creator for that background. The practice rows and the in-game NEW GAME SHALL open the free creator in practice mode, NEW CAREER in arena mode and VERSUS in versus mode. Opening SHALL hide the menu.

#### Scenario: A story character
- **WHEN** the player picks BEGIN and confirms the knight's card
- **THEN** the creator opens in story mode for background 1 with the menu hidden

#### Scenario: A practice character
- **WHEN** the player picks EXPERT on the practice page
- **THEN** the creator opens in practice mode for the expert ladder

### Requirement: Creator layout
The creator SHALL fill the window and lay out the creation form's seven records, the body editor's twelve and the hair editor's seven at unit 4, then scale every place and size by the live GUI unit as the original's rescale does (divided by 4, times the unit). The body editor SHALL sit at 4, 60, the hair editor at the form's width less 176, 32, and the preview viewport 180 in and 24 down, the form's width less 360 wide and its height less 28 high.

#### Scenario: Unit 4 on a 1024 × 768 window
- **WHEN** the creator opens at 1024 × 768 with unit 4
- **THEN** the body editor is at 4, 60, 172 × 592, the hair editor at 848, 32 and the viewport at 180, 24, 664 × 740
- **AND** BACK and NEXT are 164 × 28, 8 in from the bottom left and bottom right corners

### Requirement: Creator commits the record
NEXT SHALL show the name warning when the name is empty and otherwise switch to the equipment page; the equipment page's final button SHALL write the creator's name, gear, skills and full health into the record and start the story (BEGIN) or the practice ladder (CREATE), the player spawning with the record's physique, skin, hair and name.

#### Scenario: An unnamed character
- **WHEN** NEXT is pressed with an empty name
- **THEN** the "Please name your character" warning shows and the page does not change

#### Scenario: Begin the story
- **WHEN** a named story character's BEGIN is pressed
- **THEN** the record carries the name and full pools and the story starts at exanima01
