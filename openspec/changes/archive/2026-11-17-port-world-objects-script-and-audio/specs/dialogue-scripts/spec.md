## ADDED Requirements

### Requirement: World clock in scripts
WorldTime and the topics' times SHALL read the world clock: the seconds a save carried plus the whole seconds of game time since its load.

#### Scenario: Three seconds after a load
- **WHEN** a save carried 8684 s and 3000 ms of game time passed since its load
- **THEN** WorldTime is 8687

### Requirement: Locale and skills in scripts
The current locale SHALL be the loaded level's id in `locales.rdb` (0 for a level it does not name), and each character's six skill point words SHALL reach the script cast.

#### Scenario: The first level and a trained character
- **WHEN** exanima01 is loaded and a character has 12000 and 5000 points in two skills
- **THEN** InLocale(2) holds and SkillTecs answers 3

### Requirement: Dialogue node slots
A dialogue graph's zero node slot SHALL be empty and step 4 bytes; a slot of 1 to 4 bytes SHALL step by its size.

#### Scenario: A two-byte slot
- **WHEN** a graph of two slots holds a 2-byte slot then a zero slot
- **THEN** both nodes are empty and the graph parses
