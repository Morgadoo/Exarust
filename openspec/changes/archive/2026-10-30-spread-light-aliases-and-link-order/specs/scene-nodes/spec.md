## MODIFIED Requirements

### Requirement: Light activation
The system SHALL activate a light whose state word is below 2: state 2, its position snapshotted unless its light word skips it (height 0 in the flat snapshot mode), its sector alias found or created (owned by the sector, moved); in grid mode its cell SHALL be stored and the flood or fill reported, otherwise its alias SHALL spread along the sector's linked sectors and an alias relink be reported. A light out of the scene and the sector SHALL drop all its aliases.

#### Scenario: A flat snapshot without a grid
- **WHEN** a flickering light at (4, 2.5, 1) m in the flat snapshot mode activates in a scene without a grid
- **THEN** its snapshot is (4, 0, 1), an alias relink and a first flicker step are reported, and no cell flood

#### Scenario: A spread on activation
- **WHEN** a light activates without a grid in a sector linked to one other sector
- **THEN** the linked sector gets a moved alias carrying the light's stamp

## ADDED Requirements

### Requirement: Light link order
The system SHALL order a light's link list heaviest first by taking each entry after the head off the chain and walking it back while the entry before is strictly lighter, an unordered compare stopping the walk.

#### Scenario: Weights with a NaN
- **WHEN** the links weigh NaN, 1 and 3 in that order
- **THEN** the list runs NaN, 3, 1

### Requirement: Light alias spread
The system SHALL find a light's alias in a linked sector or make one, a light with the option word's gate bit only where its sphere reaches the sector's box; an alias with the light's stamp SHALL prune the walk, an older one SHALL be refreshed and marked moved, and the walk SHALL recurse through each sector that returned an alias, skipping busy sectors.

#### Scenario: A ring of rooms
- **WHEN** a light spreads from room 1 of rooms 1, 2 and 3 linked in a ring
- **THEN** rooms 2 and 3 each get one alias and no room stays busy

### Requirement: Published light records
The system SHALL publish the sun record's direction times its fourth lane and that lane's root, and a reach only when it differs from the published one by at least 0.01 (a NaN difference publishes), scaling the sun direction by it, halved while the mode flag is clear.

#### Scenario: A reach inside the dead band
- **WHEN** the reach 10 is published and then 10.005
- **THEN** the published reach stays 10
