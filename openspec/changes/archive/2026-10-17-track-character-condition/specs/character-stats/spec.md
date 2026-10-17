## ADDED Requirements

### Requirement: Condition word
The system SHALL keep on every fighter the original's condition word: 0 normal, 1 walking, 2 attacking, 8 combat idle, 0x20 casting, 0xC0 shocked, 0x100 weak, 0x200 down, 0x300 getting up, 0x600 dormant, 0x800 collapsing, 0xA00 dead, 0x8000 kicking, 0x10000 and 0x10004 switching weapons; and test it by the original's masks 0xAC0 (helpless), 0xA00, 0x200, 0x800 (dying) and 0x10203.

#### Scenario: Masks
- **WHEN** the masks are tested against every value
- **THEN** 0xAC0 holds shocked, down, getting up, dormant, collapsing and dead; 0xA00 the same but shocked; 0x200 down, getting up, dormant and dead; 0x800 collapsing and dead

#### Scenario: Placed corpses
- **WHEN** `play exanima01` places its dead characters
- **THEN** each corpse's condition is dead

### Requirement: Posture decision
Once each fixed tick, before the clips advance, the system SHALL write a living body's condition: a dazed one normal unless helpless or attacking; then collapsing, dead and shocked kept; getting up kept; under a balance of 0.6 (double precision) down below 0.3, else kept; an attack clip attacking; a tone under 0.85 weak; else by its clip: the kick, the combat idle (or a parry), a power, the walk or the run, a weapon switch (switched after 60 %), else normal.

#### Scenario: Thresholds
- **WHEN** a body in the combat idle has a balance of 0.6, 0.599 and 0.299 in turn
- **THEN** its condition is 8, still 8, then 0x200; at 0.3 a walker stays 1

#### Scenario: Attacks and tone
- **WHEN** a standing body plays an attack at tone 0.1, then the combat idle at tone 0.84 and 0.85
- **THEN** its condition is 2, then 0x100, then 8

#### Scenario: Knocked over
- **WHEN** `exarust --shove 60 --trace t.csv play arena` deals the player a body blow
- **THEN** the trace's condition reads normal while it falls, 0x200 once its balance is under 0.3, 0x300 from the get-up's start and normal again when the get-up clip ends

### Requirement: Condition events and readers
A get-up's start SHALL write getting up and the end of its clip normal; a killing blow SHALL write dead. Hit detection SHALL skip the dead; the daze SHALL leave a helpless or attacking fighter's clip alone; the player SHALL not be steered nor engage an attack while helpless; the duellist SHALL idle for 0xA00 and leave a dying target; the world cursor SHALL read dead, collapsing and bit 0x200 from the word.

#### Scenario: Killing blow
- **WHEN** a cut takes a fighter's yellow health under 0.01
- **THEN** its condition is dead and no blade strikes it again

#### Scenario: Dazed attacker
- **WHEN** a fighter is dazed during its attack
- **THEN** its condition stays attacking and its attack clip plays on

#### Scenario: Cursor
- **WHEN** the player's condition is getting up, then dead, in combat mode
- **THEN** the cursor is the ring while it gets up and the arrowhead once dead
