# Spec Delta

## ADDED Requirements

### Requirement: The engagement level gates the score
The position score SHALL compute the strike and approach terms only at engagement level 3; at levels 0, 1, 2 and 4 it SHALL clear both and double the close term. A live swing's danger and a recovering blow's close reduction SHALL be scored at every level.

#### Scenario: Off the fight level
- **WHEN** a place 25 cm away with reach 50 cm is scored at levels 0, 1, 2 and 4, and a side swing at phase 0.5 is scored at level 2
- **THEN** the strike and approach are 0 and the close term is twice level 3's, and the swing's danger bits stay 0x40000000

### Requirement: The foe's swing word publishes its arc
The duellist SHALL publish its foe's arc as recovering (kind 4, not sweeping) when the foe's swing word carries the cleaving bit, and committed when it carries either continuation bit. The foe's blades, reach and skill SHALL be read from the fighter under the target, and the thrust's arc test SHALL take the observer's bearing from the foe's measured origin.

#### Scenario: Swing word bits
- **WHEN** an empty-handed foe mid attack publishes with no bits, the cleaving bit, each continuation bit and the release bit
- **THEN** no bits give kind 3, sweeping and not committed; cleaving gives kind 4 and no sweep; each continuation bit commits; the release bit changes nothing
