## ADDED Requirements

### Requirement: Door order sounds
A signalled open SHALL play the first door's lock set once at its leaf, and a close and lock SHALL play each unlatched sliding leaf's lock set, at volume 0.15 + 0.1r and pitch 0.9 + 0.15r. When a leaf is shut, bit 0 of the group's object word SHALL be set.

#### Scenario: A gate closes and opens
- **WHEN** an unlatched sliding leaf is signalled to close and lock, then to open
- **THEN** its lock set plays once each time

### Requirement: Lever return word
Arming a lever's timed return SHALL write the time divided by 100 into bits 16 and up of its object word, each tick before the deadline SHALL write the time left the same way, and the deadline SHALL clear those bits, keeping the low half.

#### Scenario: A timed lever
- **WHEN** a lever with a 500 ms return is turned on and 132 ms pass
- **THEN** its word holds 5, then 3, then 0 above bit 16

### Requirement: Doors latched at a spot
An order to latch or unlatch the doors at a spot SHALL take the first placed object with a door record within 10 cm of the spot on x and on z and set or clear the latch of every door body under it and bit 0 of its word, waiting while the body is not registered yet.

#### Scenario: The physician's door
- **WHEN** the arena hub is loaded and the physician's order arrives
- **THEN** the hinged door at (−2100, 900) cm is unlatched

### Requirement: Static placement
A placed object whose placement carries bit 0x10 SHALL mark its root and every node under it level geometry; an object whose own flag marks it static SHALL mark its root only.

#### Scenario: Placement and object flags
- **WHEN** an object is placed with bit 0x10, and another carries only its own static flag
- **THEN** the first marks its whole tree, the second its root
