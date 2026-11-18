## ADDED Requirements

### Requirement: Held items carried by a placement
A placement of a holder's body SHALL carry each item it holds with the holding hand, the item keeping its pose in the hand's frame and every particle set so it moves at rest, before the held items' re-place runs.

#### Scenario: A respawn placement
- **WHEN** a holder is placed a metre away and turned a quarter turn with no re-place after
- **THEN** its held item stands in the same pose in its hand's frame as before, at rest

### Requirement: Touch through the holder
A touch that resets a sleeping holder's contacts SHALL reset the contacts of its sleeping held items too, at the next share of the holder's sleep.

#### Scenario: A blow on a sleeping corpse with its weapon
- **WHEN** a sleeping holder is touched while its held weapon sleeps with a stored level contact
- **THEN** the weapon's contacts are dropped and it wakes with its holder
