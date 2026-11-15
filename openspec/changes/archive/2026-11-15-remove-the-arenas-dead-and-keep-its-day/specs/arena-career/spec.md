## ADDED Requirements

### Requirement: Company deaths in a match
A company fighter's fatal fall SHALL log a death record with his roster name's sixteen bytes and raise the death count; a fall whose id names nobody on the roster SHALL log nothing. The log's saved records SHALL be renumbered when the fall is resolved, and the settled match SHALL take the dead off the roster and the tournament entries and queue their kits' release.

#### Scenario: A hired fighter dies
- **WHEN** the first of two hired fighters falls dead in a won Duel and the match is settled
- **THEN** his death is logged and counted, the second fighter is id 1, the event names only him and the first place is queued for the release

### Requirement: Departure before a match
The board's Begin SHALL send away the first specialist under 65000 health before the match: under 700 with a death record and one more death, else with a leaving record (0xA0000), both with his name, then the removal and the kit's release.

#### Scenario: An injured physician
- **WHEN** the board's Begin is pressed with a physician at health 500
- **THEN** he leaves the roster and the staff list, a death record names him and the death count rises

### Requirement: Kept bodies reset
A body the next round keeps SHALL be reset: yellow health takes red, the shield arm is full, the condition is 0, the standby behaviour, and for anyone but the player the AI's clocks are held 10000 ms with the first a further high share of the stepped roll times 10000. The kept opponent SHALL be reset before the continuing fighter.

#### Scenario: The opponent who won stays
- **WHEN** a reserve round follows a lost round whose fighter is dead
- **THEN** the opponent who won is reset with its clocks held from the round's start

### Requirement: Day log cleared
The day's end SHALL clear the day's log.

#### Scenario: End Day
- **WHEN** End Day is pressed with a saved record in the log
- **THEN** the log is empty

### Requirement: Hired combatant kit
A hired combatant's kit SHALL be gathered from his six loadouts (each loadout's first hand pair's set items, then its worn items but item 8, each item once) and stowed in the arsenal with the fighter-kit flag when the staff list takes him on.

#### Scenario: A combatant hired
- **WHEN** a combatant whose first loadout holds item 77 and wears 88 and 8 is hired
- **THEN** his kit starts 77, 88 and the weapons shelf holds 77 with the fighter-kit flag

### Requirement: Hire list after a load
A load SHALL clear the hire list before its chunk is read, so a save with no candidates loads an empty list, and Manage Company SHALL draw a new list when it opens on an empty one.

#### Scenario: An empty list
- **WHEN** Manage Company is opened on an empty hire list
- **THEN** the list holds ten candidates

### Requirement: Day specials saved
The day chunk SHALL keep the day's six special matches by template id, rank and flags, and the load SHALL roll each id's ranked template again at its place and put back its rank, window and flags; an id no ranked template has SHALL take the last of the seven.

#### Scenario: Round trip
- **WHEN** a day board is saved and loaded over another board
- **THEN** each match has the saved template, flags, place, rank and prize

### Requirement: Continue fixups
After an arena slot is read, a missing week's tournament SHALL be generated and the unfit entrants dropped from the pending events.

#### Scenario: An unfit manager entered
- **WHEN** a slot whose manager is entered at health 100 is loaded
- **THEN** the event has no entrant after the load

### Requirement: Won items into the arsenal
A week the company takes SHALL put the week's prize in the arsenal free of any holder, and a swept season SHALL put its four reward items there after it.

#### Scenario: A won week
- **WHEN** End Day resolves a week whose six events the company won
- **THEN** the arsenal holds the week's prize with no flags

### Requirement: Entrant health from the word
A company entrant SHALL spawn at the health its saved word gives: the maximum times the word times 1/65535 in double precision, stored as single, for red and yellow.

#### Scenario: A low word
- **WHEN** an entrant of maximum 0.3 spawns with the word 259
- **THEN** its health is 0x3a9b6702, not the single-precision product 0x3a9b6703

### Requirement: Dragged rank points
While the loadout form's rank slider is dragged, its points line SHALL show the dragged rank's loadout cost over that rank's limit; the release SHALL switch the tab.

#### Scenario: A drag to Expert
- **WHEN** the slider is dragged from the first rank to the fourth and released
- **THEN** the points line reads the fourth rank's limit during the drag and the tab is the fourth after it
