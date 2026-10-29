## ADDED Requirements

### Requirement: Placement marker registration
The level loader SHALL register every spawned model node whose name starts with `[` as a placement marker carrying the eight name characters after the `[` and the character after them, and every placed object SHALL carry its record's template beside its id.

#### Scenario: The torch holder
- **WHEN** a level places a torch holder
- **THEN** its `[0000001F]` node is a placement marker for id 0x1F

### Requirement: Lifted item snaps to a marker
While the item cursor holds an object, a marker whose hexadecimal name gives the object's id or template, whose bounding sphere the cursor's segment meets, with the ray not climbing, SHALL take the held part's frame whole and give the hover word 0x10, or 0x30 when its link character is `L`, provided the marker lies within reach.

#### Scenario: A torch over its holder
- **WHEN** a lifted torch's cursor ray passes through the holder's marker sphere within reach
- **THEN** the torch takes the marker's frame and the hover word is 0x10

### Requirement: Lift frame holds only in reach
A free placement SHALL hold only when its box fits (280 cm up, −1000 cm down) and the item's new place is within the walk-to reach of the hip; a frame that does not hold SHALL put the item back at the last placement that held, keep the hover word and count as out of reach.

#### Scenario: Out of reach
- **WHEN** the cursor places a lifted item 4 m from the player
- **THEN** the item stays at its last placement and the cursor shows the cross

### Requirement: Lift release
Releasing a lift in reach SHALL put the item down where it is. A cancelled lift SHALL be placed at its last fitting placement when its hover word has bit 1 and that placement fits, else stay on its marker (frozen for the word 0x30) when the word has bit 0x10, else go back to the frames the lift saved. A fall SHALL end a lift as a cancelled one, and a press that lifts a frozen object SHALL thaw it.

#### Scenario: A cancelled free lift
- **WHEN** a lift whose last frame held is released with the point out of reach
- **THEN** the item is put down at its last fitting placement

### Requirement: Pad analog word in combat
In combat mode the pad's stick vector SHALL replace the movement keys once its squared length, summed x, y, then z, is over 0.01 (compared in double; a NaN reads the keys), giving the analog movement word whose goal and queued step run along the vector as it was read.

#### Scenario: The stick steps
- **WHEN** the pad's stick is pushed half way to the camera's left in combat mode
- **THEN** the footwork's goal lies along that vector

### Requirement: Approach bend
The goal of a lone forward word while the body has a target SHALL be bent across the ground at the target's circle of 60·scale cm: from outside, a step that would enter it SHALL be cut where it first meets the circle when the original's angle test passes; from inside, it SHALL be pushed out to the radius when the radius is over the squared distance, and zero otherwise.

#### Scenario: Pushed out
- **WHEN** the player's chest is 5 cm from the target's and W alone lays a goal
- **THEN** the step is 60 cm straight away from the target
