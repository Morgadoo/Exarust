## ADDED Requirements

### Requirement: Doll weapon take-out
Taking a weapon out of a hand SHALL read and, when the job records, clear the record entry naming it in either hand. A scheduled take-out of the main hand whose weapon was the body's first attached weapon SHALL move the off hand's light or weapon into the record's main hand with the off hand's flags; a shield SHALL stay.

#### Scenario: Promotion after the first weapon only
- **WHEN** a scheduled take-out removes the main hand's sword while the off hand holds a dagger, once with the sword the first attached weapon and once with the dagger attached before it
- **THEN** the first leaves the dagger in the record's main hand, the second leaves it in the off hand

### Requirement: Doll off-hand take-out
Taking out the off hand's light or shield SHALL read and clear the record's off-hand entry without comparing its item. A hand that already holds the item the take-out makes room for SHALL be left alone.

#### Scenario: A shield replaces a two-hander
- **WHEN** a shield is taken while the off hand's model is a two-hander named in the record's main hand
- **THEN** the main hand's entry is cleared and the off hand holds the shield

### Requirement: Swapped sets dress the hands
After a weapon-set swap that changes a hand, the moveset's hands SHALL follow the record's out set: the main hand's weapon, a second weapon or shield in the off hand, and an off-hand item with a light section as the off hand's light. A carried torch outside the record SHALL stay in its hand.

#### Scenario: The torch survives a swap
- **WHEN** a body carrying a torch in its main hand swaps to a set holding a sword and a shield
- **THEN** its main hand keeps the torch and its off hand holds the shield
