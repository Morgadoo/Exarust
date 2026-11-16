## MODIFIED Requirements

### Requirement: Header camera pair
Both writers SHALL store the view camera's place (the translation of the viewport camera node's matrix, the eye, in centimetres) and the camera controller's current zoom word in the header after the locale. The running game SHALL keep the pair on the chase camera's eye and shown zoom each frame; without a camera a save SHALL write the origin and zoom 1. Both readers SHALL read the pair and drop it: a read session SHALL keep the origin and zoom 1.

#### Scenario: Write only
- **WHEN** a story session with camera (123.5, −45, 678.5) and zoom 1.75 is saved and read back
- **THEN** the header holds those values and the read session has the origin and zoom 1

#### Scenario: A live save takes the chase camera
- **WHEN** a save runs while the chase camera's eye stands at (69.8125, 15.75, 32.5) m with its starting zoom
- **THEN** the header holds the place (6981.25, 1575, 3250) and the zoom 1300

## ADDED Requirements

### Requirement: Journal page id words
Each journal page SHALL carry an id word that only the save writer reads. A page the game makes (the opening page, an added page, the erase's reset of the last page) SHALL carry 0; a page a save load puts back SHALL keep its saved word; an erase SHALL move the later pages down with their words. The writer SHALL write each page's word.

#### Scenario: A loaded page keeps its word
- **WHEN** the in-game journal holds a new page and a page with word 0x51, is saved and the save is loaded
- **THEN** the journal chunk holds words 0 and 0x51 and the in-game journal's second page carries 0x51

### Requirement: Door arrival save landing
A door's arrival SHALL restart the player body's stance clip with no blend and no capture, set its tone to full, clear its condition word, condition flags and locomotion flags, probe the floor 300 cm down from 100 cm over the placement (a miss keeps the lifted start), and put the pelvis at 108.18 − 11.1 × the record's height (double precision) plus 13 cm in combat mode or 5 cm over that floor, the held items put back.

#### Scenario: Landing out of and in combat mode
- **WHEN** a body of height 0 arrives through a door, then arrives again in combat mode
- **THEN** its pelvis is asked 113.18 cm, then 121.18 cm, over the probed floor, and its stance clip, condition and flags are reset

### Requirement: Match return reads the saved hub
The settled arena match's return to the hub SHALL read the hub from the current arena slot's saved `arenahub` section when the save holds one, and from the archive otherwise. Other arena loads SHALL leave the archive's level.

#### Scenario: Return after a save that left the hub
- **WHEN** the match returns to the hub and the current arena slot's save holds an `arenahub` section
- **THEN** the hub load reads that section
