## ADDED Requirements

### Requirement: Match fighters think for themselves
Every generated match fighter (opponent, hire-list rival, the Beast, company entrant) SHALL carry a brain with relations and an AI clock held until the match's action grace, so its target pass picks its foe among the other sides with or without the player.

#### Scenario: A spectated duel is fought
- **WHEN** a hired combatant alone is entered against one opponent
- **THEN** each takes the other as its foe after the grace and they fight

### Requirement: Fixed spectate camera
A spectated match's start SHALL put the camera at the arena's spawn spot raised by 700, 1180 and 1100 cm, looking at the spot, until a double click picks a character to follow.

#### Scenario: The start's pose
- **WHEN** a spectated match starts at a spot (1, 2, 3) m
- **THEN** the camera stands at (8, 13.8, 14) m looking at the spot

### Requirement: Saved hire list
An arena save SHALL write each hire-list candidate as an unused word and its character export (name, role word, skills, worn and held items, six loadouts), and a load SHALL rebuild the list from them.

#### Scenario: Save and reload
- **WHEN** a career is saved and its slot loaded
- **THEN** the candidates' names, professions, skills, outfits and loadouts are those saved

### Requirement: Saved week's events
An arena save SHALL write the week's prize and, per event, its template id, index, weapon row and the word after it; a load SHALL copy each event's template and set its rank window to 5 and the index.

#### Scenario: Save and reload the week
- **WHEN** a career is saved and its slot loaded
- **THEN** the week's prize and each event's template, index and weapon row are those saved

### Requirement: Saved week winners
An arena save SHALL write 1 for each season week the company took and keep a rival winner the save held for the others; a load SHALL count a week whose winner is 1 as won.

#### Scenario: Two weeks won
- **WHEN** a career that took weeks one and three is saved and loaded
- **THEN** weeks one and three are won and the others are not
