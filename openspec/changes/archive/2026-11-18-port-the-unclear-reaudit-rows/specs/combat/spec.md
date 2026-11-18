## ADDED Requirements

### Requirement: Strike speed from the swing tracker
A weapon contact's strike speed SHALL be the striker slot's tracked speed: the root of the tracker's speed squared when that is over the epsilon, else the squared value as it is.

#### Scenario: A tracked swing
- **WHEN** the striker's tracker holds a speed squared of 16
- **THEN** the strike speed is 4 cm a tick

### Requirement: Stab speed along the weapon
A stab's strike speed SHALL be twice the tracked velocity along the striking body's long axis, with no root, so it may be negative.

#### Scenario: A stab moving across the blade
- **WHEN** the tracked velocity is (3, 4, 0) and the blade lies along y
- **THEN** the stab's speed is 8

### Requirement: Fist measured by its hand
A fist's contact SHALL be measured along its hand's local x, negated for the right hand.

#### Scenario: The right fist
- **WHEN** the right fist strikes
- **THEN** its contact is measured along the hand's minus x

### Requirement: Burn groan
A due burn crackle at a timer of 19 or under SHALL ask the burning body's voice for its groan (line 2) at the crackle's volume and pitch, at the body.

#### Scenario: A quiet crackle
- **WHEN** a burn at timer 5 comes due
- **THEN** a groan is asked of the body's voice
