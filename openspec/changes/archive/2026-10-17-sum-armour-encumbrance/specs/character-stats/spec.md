## ADDED Requirements

### Requirement: Encumbrance
The system SHALL sum a character's encumbrance at spawn as the original's armour pass does: 0.0025 per unit of each worn item's encumbrance (an unpowered golem suit adds 2), less 0.025 for each articulated piece touching another, less 0.3 per Manoeuvring in Armour tier, none below 0.001; then the held shield's (2 − weightless) × bulk / 4, less 0.3 with Mobility, when positive; at most 1.

#### Scenario: The duellist's outfit
- **WHEN** the duellist spawns in its leather jacket, trousers, boots and full helm
- **THEN** its encumbrance is 0.175 and its step reach 77.2 cm

#### Scenario: A plate set
- **WHEN** the player spawns with `--outfit fptdhrnsa,fptdhelma,fptdlegsa`, a tourney harness, helm and leggings of one articulated set
- **THEN** its encumbrance is 0.625: 0.7 less 0.025 for each of the three touching pieces

#### Scenario: Techniques and a shield
- **WHEN** a record wears a mail shirt of encumbrance 40 and holds a sword and a round shield of bulk 0.6
- **THEN** its encumbrance is 0.4; 0.3 with Manoeuvring in Armour I; 0 with Mobility as well

#### Scenario: Golem suit
- **WHEN** a character wears an unpowered golem suit
- **THEN** its encumbrance is 1, whatever its Manoeuvring tier

#### Scenario: Story levels
- **WHEN** every living character of the installed story levels spawns
- **THEN** 386 of the 451 weigh nothing, 27 hold a shield, and Elestan (0.77, with a round shield) and Demian (0.67) are the most encumbered

#### Scenario: Arena actors
- **WHEN** the arena's actors spawn in their own items
- **THEN** opponent07 weighs 0.475 (0.175 of armour and a round shield's 0.3), opponent13 0.07 (Manoeuvring II takes its armour off, its buckler stays) and the demon nothing

### Requirement: Encumbered movement
A character's step reach SHALL shrink by 20 % per unit of its encumbrance, and the player's run blend SHALL rise at most to 1 − 0.3 × its encumbrance.

#### Scenario: Step reach
- **WHEN** a human of scale 1 spawns at encumbrance 0.5
- **THEN** its step reach is 72 cm

#### Scenario: Running in plate
- **WHEN** the player wearing the 0.625 plate set runs to a far cursor
- **THEN** its run blend stops at 0.8125 and it runs at about 3.2 m/s instead of 3.6 m/s
