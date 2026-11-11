## ADDED Requirements

### Requirement: Cast after the first place's contact pass
The NPC's cast SHALL be weighed at the first place of the search, after that place's contact pass, and only while the fighter still has a foe: a pass that drops the foe, or a search that starts past the first place, SHALL cast nothing that tick.

#### Scenario: Nothing presses at the first place
- **WHEN** a fighter with usable powers weighs its first place and no contact passes the floor
- **THEN** its foe is dropped and it casts nothing that tick

### Requirement: Footwork before the cast
The footwork's change cost SHALL read the attack drive before a cast clears it, and its draws SHALL come before the cast's; a cast SHALL clear the attack drive for every place, the first included.

#### Scenario: A cast at the first place
- **WHEN** a fighter casts at the first place with an attack drive of 3
- **THEN** the footwork's change used 3 and the places weigh an attack drive of 0
