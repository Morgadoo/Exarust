## ADDED Requirements

### Requirement: Worn effect bytes written back
Every worn, apparel or healing tick that moves an item's counter byte SHALL write it into that item's apparel body in the live object database, so the item catalogue, the armour pass and a save read the spent charge.

#### Scenario: A spent golem suit
- **WHEN** a golem suit's tick takes its byte from 0xfd to 0xff
- **THEN** the suit's record holds 0xff and the catalogue reads it unpowered

### Requirement: Golem suit switch
The golem suit's registration while powered and its tick's switch on SHALL set the body's golem-suit byte and its standing stance factor 0.75; the switch off SHALL clear the byte and restore the race's stance factor. Either switch SHALL run the body's equipment refresh: the armour pass, the step reach and the view's encumbrance dots.

#### Scenario: The suit runs out
- **WHEN** a powered suit's tick turns it off
- **THEN** the stance factor is the race's, and the next pass adds the dead suit's two to the encumbrance, held at 1

### Requirement: Gauntlets read their stored share
The gauntlets' tick SHALL compare the body's stored weightless share, which the armour pass registered, with its hysteresis, and its switches SHALL run the equipment refresh.

#### Scenario: A pair already unpowered
- **WHEN** a pair registered with share 0 steps its byte to 0xff
- **THEN** no switch runs and the byte is written back
