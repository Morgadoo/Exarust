## ADDED Requirements

### Requirement: The arena's return runs behind the transition screen
The end screen's Continue after a match's last round SHALL end a tournament's bracket, close the end menu and open the transition screen with the music fade on; the match SHALL be settled and the hub's level asked for only at the screen's black, once. Without the flow the match SHALL be settled at once.

#### Scenario: A won match's Continue
- **WHEN** the Continue is pressed on a won match's end screen with the flow running, on 16 ms frames
- **THEN** nothing is paid during the fade, the hub is asked for once about 38 frames later with the win paid, and the screen closes on the playing phase

### Requirement: A dead manager leaves behind the transition screen
A dead manager's Continue SHALL open the transition screen with the music fade on over the leave to the title, settling nothing and asking for no hub; without the flow it SHALL end the match at once.

#### Scenario: A dead manager's Continue
- **WHEN** the Continue is pressed with the manager dead and the flow running
- **THEN** the title's mode comes at the screen's black, about 38 frames later, and no arena exit is sent

### Requirement: The round stagings write the health words back
The next round's and the bracket round's stagings SHALL first write every company body's health word back into its record, as the return does.

#### Scenario: A bracket round's write-back
- **WHEN** a team bracket round is stood with a company fighter at an eighth of his health and a dead one
- **THEN** the first's record holds the word of an eighth and the dead one's holds 0

### Requirement: A later bracket round keeps the company's bodies
A later bracket round SHALL keep the company's living bodies, the player's included, in entry order, re-dress them and reset them on the company side's places, body `k` at place `k` of the spread the side's member count gives; a dead company body SHALL leave with the opponents, and only the other sides SHALL be generated.

#### Scenario: A team bracket round
- **WHEN** a three-a-side bracket round follows with the player, a living fighter and a dead fighter on the company's side
- **THEN** the dead fighter and the opponents go, the player and the fighter are reset on the side's first two places in that order, and three opponents of one side are generated

### Requirement: The arsenal's shelves fill its columns
The arsenal's open SHALL load the first shelf's five boxes into its five columns. A shelf button that goes down SHALL store the columns into the shown shelf's boxes and load the shelf it names; the close SHALL store the shown shelf. Every entry SHALL keep its item, flags and place.

#### Scenario: Two shelves
- **WHEN** the arsenal opens over two filled shelves, the second shelf button is clicked and Escape closes it
- **THEN** the first shelf shows on the open, the second after the click, and the stores come out as they went in

### Requirement: The arsenal's controls answer a click
A left release over an arsenal control SHALL run its callback, the control drawn last taking it where two overlap: the arrows SHALL scroll one column (the left one only past zero) and lay out, and the toggle SHALL show the inventory while it is down.

#### Scenario: Arrows and toggle
- **WHEN** the right arrow, the left arrow and the toggle twice are clicked
- **THEN** the scroll goes to 1 and back to 0, and the inventory hides and shows again

### Requirement: The board's item slots open the item's card
A click on one of the board's item slots holding an item (the tournament prize, a season reward) SHALL ask for that item's card at once with no flags; an empty slot SHALL ask for nothing.

#### Scenario: Season rewards
- **WHEN** the season tab's third reward slot holding an item is clicked, then the empty second
- **THEN** one immediate card of that item is asked for, then none

### Requirement: The hub entry starts the manager's entry motion
Before the hub entry places the manager it SHALL start his motion 0x13 with no blend and no capture.

#### Scenario: The entry
- **WHEN** the manager's body appears in the entered hub holding a captured pose
- **THEN** it plays motion 0x13 with no capture and no pending clip, and its living placement is asked for

### Requirement: A windy match start wakes the level's cloths
When the match's first round is stood and the scene's wind widened to double is over 0.1, every cloth of the level SHALL have its sleep countdown raised to 32 at the next fixed step; a calmer wind or a not-a-number SHALL wake none.

#### Scenario: Wind and calm
- **WHEN** a first round is stood with a cloth at countdown 3, once with wind 0.5 and once with wind 0.05
- **THEN** the cloth's countdown is 32 after the first and stays 3 after the second
