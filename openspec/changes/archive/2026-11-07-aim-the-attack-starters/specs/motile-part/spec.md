## ADDED Requirements

### Requirement: Aimed side attack pick
A side attack's start SHALL pick the side's first clip when the fighter crouches. Otherwise a fighter with stance class 3 and Ward, or Synchrony when the start does not chain, or with a weapon in each hand, SHALL take the clip answering the parry it would pick against its foe's attack: Left or Right the second clip, Up the first, a low parry the third, a shield fighter's other side swapping the second and third. It SHALL then count the foe's motion as countered.

#### Scenario: Ward against a low blow
- **WHEN** a Ward shield fighter starts a main-side attack while its foe swings a blow it would parry Left Dn
- **THEN** it plays Attack Up and counts the foe's motion as countered

### Requirement: Recent parry steers the side pick
Without an answer, a shield or two-weapon fighter's own parry pick of the last 500 ms SHALL steer the side clip by the same table (for two weapons only by the side's own parries and Up), countering nothing; failing that the clip SHALL be drawn at random, never the side's last pick.

#### Scenario: Plain shield
- **WHEN** a shield fighter without Ward or Synchrony starts a side attack with no parry in the last 500 ms
- **THEN** the clip is drawn at random

### Requirement: Countered motion at every attack start
Every attack start SHALL write the foe's motion it counters into the fighter's parry state, none when it counters nothing. An overhead SHALL counter on a Left or Left Dn parry and a stab also on Up, for a shield fighter with Synchrony or a fighter with two weapons.

#### Scenario: Unaimed start
- **WHEN** a fighter that countered a motion starts an attack that is not aimed
- **THEN** its countered motion is none

### Requirement: AI side attacks pick their clip
The AI's side attack SHALL take its clip from the side starter's pick, not always the side's first clip.

#### Scenario: Repeated main-side attacks
- **WHEN** a duellist without a shield starts main-side attacks in a row
- **THEN** no two in a row play the same clip

### Requirement: Chained motion for the parry chooser
A chained attack SHALL keep the attack it chained from, and the parry chooser SHALL read that motion while the swing carries the continuation.

#### Scenario: Remise chain
- **WHEN** a foe chains Attack Combo 1 from Attack Mid
- **THEN** a parry against it is picked for Attack Mid

### Requirement: Two weapons' immediate counter
A hard hit on a two-weapon fighter's striking weapon that would interrupt its side attack past 0.6 of the clip SHALL instead chain the other side's attack, when the attack state holds none of the hold bits.

#### Scenario: Main side struck hard
- **WHEN** a two-weapon fighter's Attack Mid weapon is struck hard past 0.6 of its clip
- **THEN** it chains an other-side attack and is not interrupted

### Requirement: Shield movesets from techniques
A shield holder SHALL fight with Shield C2 when it has Ward, else Shield C1 when it has Synchrony, else the plain shield moveset.

#### Scenario: Ward and Synchrony
- **WHEN** a shield holder has both Synchrony and Ward
- **THEN** its moveset is Shield C2
