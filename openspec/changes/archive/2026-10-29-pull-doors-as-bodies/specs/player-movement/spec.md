## ADDED Requirements

### Requirement: Door press
A drag start on a door's hinged leaf SHALL rattle and refuse when the leaf is latched and not being unlocked; otherwise it SHALL add a link from the leaf point 100 cm from the door node's origin, taken at the hit's height, toward the hit, with slack 1 cm and strength 0.003, solved in the tick's first three passes only, and mark the leaf held. A door that is not clickable SHALL be nothing to the hand.

#### Scenario: A latched leaf
- **WHEN** the hand pulls a latched leaf for 300 ticks
- **THEN** it stays shut

#### Scenario: A door that is not clickable
- **WHEN** the pick names a leaf whose door is not clickable
- **THEN** the hand takes nothing

### Requirement: Door pull each frame
While the hand pulls a leaf, each frame the link's anchor SHALL be the point 100 cm from the door's origin toward the cursor's point on the plane at the hit's height, waking the leaf to 256; while the player's hip is farther than 350 cm from the door the link SHALL be disabled instead. The release, a fall or any forced release SHALL remove the link and the hold.

#### Scenario: Pulling a shut leaf
- **WHEN** the hand pulls a shut, unlatched leaf toward its open side
- **THEN** it leaves the shut state within 60 ticks and swings more than 0.5 rad, never past its stop, and the release removes the link

#### Scenario: Out of the door's reach
- **WHEN** the pull is suppressed for a frame
- **THEN** the link's disabled bit is set and its anchor kept, and the next frame clears it
