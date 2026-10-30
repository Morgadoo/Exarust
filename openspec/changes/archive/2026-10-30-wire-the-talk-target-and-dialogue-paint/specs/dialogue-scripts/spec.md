## ADDED Requirements

### Requirement: Talk candidates
On the frame the Talk key is pressed with no dialogue open, the candidates SHALL be the player's nearby characters: those in the player's room or a room behind one of its exits, seen from the player, within 30 m, the sixteen nearest; collapsing and dead ones (condition bit 0x800) SHALL be skipped.

#### Scenario: Sixteen within 30 m
- **WHEN** twenty characters stand 1 to 20 m away and one at 30.01 m, with no level rooms
- **THEN** the sixteen nearest are candidates and the far one is not

### Requirement: Talk target
The character talked to SHALL be the first candidate whose chest point, projected through the camera's rigid inverse view and viewport projection, lies at a squared pixel distance from the cursor below every earlier one's and below 3.4e38; an unordered distance SHALL never win and there SHALL be no facing test.

#### Scenario: Nearest on screen
- **WHEN** chests show at (900, 360), (700, 400), a collapsing one at (640, 360), a dead one at (641, 360) and an unordered one, with the cursor at (640, 360)
- **THEN** the one at (700, 400) is talked to

#### Scenario: No range on screen
- **WHEN** the only chest shows at (−5000, 9000)
- **THEN** it is still talked to

### Requirement: Reply panel form
The reply panel SHALL be made on the first talk, 400 by 120 pixels, centred across its parent and 120 + 32 pixels above the parent's bottom, and reused afterwards; it SHALL be shown while the engine's panel is open, also with an empty list, and hidden when that closes. The engine's pick turning on SHALL hold the picked entry for 1000 ms.

#### Scenario: First talk
- **WHEN** the player first talks on a 1280 × 720 window and the list needs 224 pixels
- **THEN** the panel sits at x 440, top 568, and its height eases from 120 toward 224, reaching 172 half way through the 500 ms

### Requirement: Reply panel pointer
The pointer over the shown panel SHALL hover the entry whose band (from one unit down, each `lines × 4 + 1` units tall) lies under it, clear the hover off the panel, count as over the GUI, and a left click released over a hovered band SHALL pick that entry.

#### Scenario: Clicking the second band
- **WHEN** a two-entry panel at top 568 is clicked at 30 pixels below its top
- **THEN** entry 1 (the second) is picked

### Requirement: Dialogue paint
After the open screens, every showing bubble SHALL be placed and drawn, a character's over its body position lifted 120 cm, then the shown reply panel, both in the main GUI font of the window's level; nothing SHALL run while there is no bubble and the panel is hidden.

#### Scenario: A reply bubble
- **WHEN** the partner answers while the panel is shown
- **THEN** one frame draws the bubble's box and text, then the panel

### Requirement: Reply panel draw
The panel's draw SHALL ease its height, fill a rounded box of radius 2 at alpha 160, and write each row's "N." one unit in and its wrapped text six units in: the hovered or held row in the highlight group's ninth ink with its alpha raised by half again, the others in the label group's ninth (its fifth while a pick is held).

#### Scenario: A hovered row
- **WHEN** a two-row panel is drawn with the first row hovered and the highlight ink's alpha 0x64
- **THEN** the box is 400 × 120 at alpha 160, "1." starts at the panel's x + 4 with its bottom 20 pixels below the top, and the first row draws at alpha 0x96

### Requirement: Session player
The player body's character id SHALL be the session's player, and a level character sharing that id SHALL stay out of the script cast and out of the Talk target.

#### Scenario: A shared id
- **WHEN** a level character and the player body both carry id 1
- **THEN** the cast's entry 1 holds the player's name and roles

### Requirement: Story companion
A new story in `exanima01` SHALL place the unknown background's companion, the `xac_derrin` actor template, at the story start's companion point (5808, 0, −4075) cm, under the character id after the level's records.

#### Scenario: Derrin greets the unknown
- **WHEN** a new player (Player, Human, XA_Unknown) talks to Derrin (Human, Activity Wait, XA_Derrin_Meet)
- **THEN** "Who are you?" is offered, and picking it brings Derrin's "My name's Derrin. I'm just a farm hand." and two replies

## MODIFIED Requirements

### Requirement: Reply panel sync
The reply panel SHALL be rebuilt from the engine's entries only when they or the narrator flag change: each entry measured at the panel width less eight units, the `[Other]` toggle (topic 0xFFFF) appended with zero lines, then laid out; an emptied engine list SHALL clear the panel's rows and keep it shown, and a closed engine panel SHALL hide and clear it.

#### Scenario: An unchanged list keeps its animation
- **WHEN** a one-entry panel is synced at 1000 ms and again unchanged at 1200 ms
- **THEN** the height animation still ends at 1500 ms

#### Scenario: A conversation step
- **WHEN** the engine's list is emptied while its panel stays open
- **THEN** the panel stays shown with no rows
