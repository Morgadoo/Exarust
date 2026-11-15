## ADDED Requirements

### Requirement: Placed bodies settle
A body given the settling state SHALL take a tick's gravity and keep none of its velocity for the given number of ticks. A character the scene takes back SHALL settle for 83 ticks unless it is dormant. The story respawn's player SHALL settle for 16 ticks once it is placed at the portal.

#### Scenario: A foe comes back into the scene
- **WHEN** a character that is not dormant changes from dormant presence to entering
- **THEN** its body settles for 83 ticks

#### Scenario: A dormant statue comes back
- **WHEN** a dormant statue's presence changes to entering
- **THEN** its body does not settle

#### Scenario: The story respawn
- **WHEN** the story respawn places the player at the portal once `exanima05` is in
- **THEN** the player's body settles for 16 ticks

### Requirement: Body freeze
The freeze SHALL clear the asleep and deep states, set the frozen and kinematic bits, place the parts at the given transforms when there are some, set every particle set to the current one, empty its contact offsets and set the countdown to 2. The item cursor's lock release SHALL freeze the item where it is.

#### Scenario: An item is locked on a marker
- **WHEN** the cursor releases an item with the lock
- **THEN** the item stays where it is with no velocity and its countdown is 2

### Requirement: Kinematic bodies skip the level
A kinematic body SHALL be left out of the level detection while it still meets other bodies. The thaw SHALL clear the frozen and kinematic bits.

#### Scenario: A frozen item touches the floor
- **WHEN** a frozen item's part reaches into the floor
- **THEN** the level detection finds no contact for it until it is thawed

### Requirement: Held items share the holder's park
Before the solver, a held item whose grip is linked SHALL be parked when its holder's body is parked and unparked when it is not. A released grip SHALL keep the item's own mark.

#### Scenario: A statue holding a sword is parked
- **WHEN** a dormant statue's body is parked out of view
- **THEN** the sword it holds is parked in the same step, and unparked with it
