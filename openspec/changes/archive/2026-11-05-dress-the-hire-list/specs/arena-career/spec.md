## ADDED Requirements

### Requirement: Dressed candidates
With the object database read, each hire-list candidate SHALL be dressed after its record from the same sequence: a combatant by its level with a twentieth of it as the height bias, its six loadouts copies of that kit with those of the ranks below its level trimmed to their rank; a physician from the profile 0x14, 0, 100, 10, 10 and a merchant or trainer from 4, 0, 100, 10, 10, holding nothing. The listed price SHALL add four fifths of the outfit's value (each worn and held item's value word as a shop line), and the hire SHALL keep the outfit and loadouts.

#### Scenario: The outfit is priced
- **WHEN** a candidate wears items worth 100 and holds one worth 12 in value words
- **THEN** its price is the listed price of its skills with those two words, and the hired member keeps the outfit

### Requirement: Hired specialist placement
A merchant or physician hired during a hub visit SHALL stand at its spot at once, as one employed at the hub's entry does, and only once per visit.

#### Scenario: A physician is hired in the hub
- **WHEN** the hub stands its shopkeeper and a physician joins the roster
- **THEN** one body is asked for the physician

### Requirement: Tournament day entries
When the day end reaches a tournament day, the unfit entrants SHALL be dropped from the pending rows, then each event entry that names someone SHALL be cleared when it names fewer than the event's fighters per side, or, for an event that arms its fighters, when its first entrant's weapon is not in the event's weapon row.

#### Scenario: A short entry
- **WHEN** an event of two fighters a side names one entrant on the tournament day's end
- **THEN** its entry is cleared, while a full entry of an unarmed event stays
