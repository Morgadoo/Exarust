## ADDED Requirements

### Requirement: Beast record
The Beast SHALL be spawned from its own record: the Ogre race, a combat skill of 100, the tier's main-hand item (0x44 for tiers 1 and 2, 0x1f6, 0x207 or 0x21e for tiers 3 to 5) and, for tiers 3 to 5, its four worn pieces in order, a voice pitch of 0.9 + 0.17 times the smoothed first draw, voice 0, 50000 experience spent on the profession grid, and personality −100, 100, 0.

#### Scenario: A tier 4 Beast
- **WHEN** a tier 4 Beast record is written
- **THEN** its race byte is 3, its main hand holds 0x207, it wears 0x210, 0x20e, 0x20f and 0x213, and its skill rows hold 50000 experience in all

### Requirement: Clock reseed of generated bodies
Before each generated opponent or Beast record the shared random sequence SHALL restart from the performance counter's low 32 bits with their bytes reversed, and later draws SHALL continue from where the generation left it.

#### Scenario: A row-armed hire side
- **WHEN** a hire side armed from an event row is generated
- **THEN** its weapon is the row slot picked by the first step after the random NPC pass, before the outfit pass

### Requirement: Rival company per tournament team
A tournament side of team 2 or above SHALL wear the hire profile's forced outfit of the company the week's field names at slot team − 2, with the event's tier as the profile's level byte, in the first round, the launch and the bracket's later fights; the field SHALL be drawn when its first company is 0 and saved with the tournament.

#### Scenario: Team 6 of a drawn field
- **WHEN** the field begins 15, 17, 4, 1, 13 and team 6 is generated at tier 2 for an event that arms its fighters
- **THEN** its profile flags are 0x0f, its level byte is 2 and its company bytes are 97, 20, 0
