## MODIFIED Requirements

### Requirement: Character locale and material order
The system SHALL look a placement whose reference carries the world bit (0x40000000) up in the world character database by its low 28 bits, and any other placement in the level's own records; a reference its store lacks SHALL place nobody. A world character SHALL be created only in the level whose locale its record stores (a level's own records always). The system SHALL shade a surface with a library material over a file's own record of the same name.

#### Scenario: Locale filter
- **WHEN** a level places a world-database character whose stored locale is another level's
- **THEN** that character is not created there

#### Scenario: A world reference with a local twin
- **WHEN** a level places world reference 0x40000004 and holds a local record numbered 4, and the world store has no record 4
- **THEN** nobody is placed for the world reference
