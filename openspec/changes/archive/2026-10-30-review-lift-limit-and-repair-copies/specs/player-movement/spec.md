## MODIFIED Requirements

### Requirement: Item use marks and timed values
A repair SHALL mark the item or its container entry with 0xFF0000 and both pours with 0xC80000; the elixir's sound SHALL play only with the potion. The timed list SHALL write a section only while its byte lowers or the entry arrives; it SHALL try the apparel (0x1000) section first and SHALL skip the weapon (0x4000) section in a frame where the apparel section exists and is not written, testing the weapon section when the apparel byte was written or the item has no apparel section.

#### Scenario: Usable section not lowered
- **WHEN** a repair entry's value is 20 and the apparel section's byte is 10 while the weapon section's is 90
- **THEN** nothing is written that frame

#### Scenario: Apparel section written
- **WHEN** a repair entry's value is 50 and both sections' bytes are 100
- **THEN** the apparel section and then the weapon section are written

### Requirement: Item cursor placement
The system SHALL place a lifted item at the cursor ray's level hit, the held item left out of the pick, only when the hit is strictly over the height the player stands at (a NaN height or a hit at that height taking the plane), else where the ray crosses that height; a ray climbing more than the float epsilon SHALL place nothing whether or not it hit. The item SHALL be placed whether or not its box fits.

#### Scenario: A floor under the player's height
- **WHEN** a player standing at 170 cm lifts an item over a floor hit at 0 cm
- **THEN** the item is placed where the ray crosses 170 cm

#### Scenario: Box out of range
- **WHEN** the placement point puts the item's box above 280 cm
- **THEN** the item is still placed there and the box test answers no

### Requirement: Game mode registration
The system SHALL register the Character mode and then the Spectate mode, each id the mode table's length after its append plus two, each record holding its zero-padded sixteen-byte name; Character SHALL name the start and end of play as its hooks and Spectate its own pair.

#### Scenario: Empty table
- **WHEN** the table has no entries
- **THEN** Character gets id 3 and Spectate id 4

### Requirement: Lift frame holds only in reach
A free placement SHALL hold only when its box fits (280 cm up, −1000 cm down) and the item's new place is within the walk-to reach of the hip; a frame that does not hold SHALL put the item back at the last placement that held (the lift's own frame before any held), keep the hover word and count as out of reach. The last placement SHALL be the held part's frame.

#### Scenario: Out of reach
- **WHEN** the cursor places a lifted item 4 m from the player
- **THEN** the item stays at its last placement and the cursor shows the cross

## ADDED Requirements

### Requirement: GUI drop placement limit
A GUI drop SHALL keep the cursor ray's hit when its height is not under −100000 cm (a NaN height too), else take the point where the ray crosses that height, testing the climb only then.

#### Scenario: A climbing ray over a hit
- **WHEN** a GUI drop's ray climbs and hits at 50 cm
- **THEN** the drop takes the hit

### Requirement: World use cursor lift
A double click on an unmarked placed object whose usable class is 1 SHALL close the item card and lift the use cursor with the object's tagged id, slot and panel zero and the usable kind, leaving the object in the scene.

#### Scenario: A level key
- **WHEN** the player double-clicks a level-placed key whose tagged id is 0x80000021
- **THEN** the use cursor is up for item 0x80000021 and the key stays where it was

### Requirement: GUI drop record
A GUI drop's spawned node SHALL carry the game record's template word, usable and container sections and mechanism records, as a placed object does; an item id carrying a tier tag SHALL keep only its id word and condition.

#### Scenario: A level-tagged pack item
- **WHEN** a pack item whose id is 0x40000007 is dropped into the world
- **THEN** the node takes no record's sections

### Requirement: Placement sweep support pull-back
The sweep's per-sphere pull-back SHALL be, for a sphere with `plane + radius − centre·w′` ordered under zero, that gap over `w·dir`, where the plane term is `w·up − offset − position·w` and `w′` is `w` dotted with each of the frame's axes; any other sphere SHALL be pulled back by zero.

#### Scenario: A turned frame
- **WHEN** w and up are (0, 1, 0), dir is (0, −1, 0), the frame is turned a quarter about z and the sphere sits at (2, 0, 0) with radius 0.5
- **THEN** it is pulled back 0.5
