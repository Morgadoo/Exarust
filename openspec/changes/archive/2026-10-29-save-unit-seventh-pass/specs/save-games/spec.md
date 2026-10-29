## ADDED Requirements

### Requirement: Arena item classification
The arena load's id pass SHALL classify an item from its object record's sections: an unknown id SHALL go on no shelf; a body section SHALL put it on the armour shelf when the body's armour type byte is 3 to 6 and on the clothing shelf otherwise; without a body a weapon section (`0x4000`) SHALL put it on the weapons shelf, else a shield section (`0x8000`) on the shields shelf, else on no shelf. Only the sections' presence SHALL count.

#### Scenario: Body wins
- **WHEN** a record has a body with armour type 6 and a weapon section
- **THEN** the item goes on the armour shelf

#### Scenario: Short weapon section
- **WHEN** a record without a body has a four-byte weapon section that does not parse
- **THEN** the item goes on the weapons shelf
