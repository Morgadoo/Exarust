## MODIFIED Requirements

### Requirement: Posture decision
In each living character's posture stage, before its clip advances, the system SHALL write its condition: a dazed one normal unless helpless or attacking; then collapsing, dead and shocked kept; getting up kept; under a balance of 0.6 (double precision) down below 0.3, else kept; an attack clip attacking; a tone under 0.85 weak; else by its clip: the kick, the combat idle (or a parry), a power, the walk or the run, a weapon switch (switched after 60 %), else normal.

#### Scenario: Thresholds
- **WHEN** a body in the combat idle has a balance of 0.6, 0.599 and 0.299 in turn
- **THEN** its condition is 8, still 8, then 0x200; at 0.3 a walker stays 1

#### Scenario: Attacks and tone
- **WHEN** a standing body plays an attack at tone 0.1, then the combat idle at tone 0.84 and 0.85
- **THEN** its condition is 2, then 0x100, then 8

#### Scenario: Knocked over
- **WHEN** `exarust --shove 60 --trace t.csv play arena` deals the player a body blow
- **THEN** the trace's condition reads normal while it falls, 0x200 once its balance is under 0.3, 0x300 from the get-up's start and normal again when the get-up clip ends

#### Scenario: Stagger
- **WHEN** a body off balance staggers
- **THEN** its condition is 0x300 from the stagger's start until its get-up clip ends
