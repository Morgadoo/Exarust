## MODIFIED Requirements

### Requirement: Fist strikers
The system SHALL give an unarmed fighter's hands strikers built from its race's fist or claw records, each a segment from the hand's origin across its box. With the fists, claws or two weapons only the motion's hand SHALL strike: the right in the main side, stab and overhead (both claws in a clawed overhead), the left in the other side; else every held weapon strikes and fists do not. A fist SHALL meet a part where its knuckles touch it.

#### Scenario: Punch lands
- **WHEN** `exarust --still-opponent --at 0,0,-1.1 --press LMB@120-200 play arenao1` runs (the button held)
- **THEN** the right straight lands on the duellist with the right fist, the arm fully out, and takes its yellow health

#### Scenario: Left straight strikes with the left fist
- **WHEN** the unarmed player's other-side attack drives both fists into a target
- **THEN** only the left fist strikes, short of its 17.66 cm far end

#### Scenario: Armed
- **WHEN** a fighter holding a sword attacks with its fists' strikers still on its hands
- **THEN** only the sword strikes

## ADDED Requirements

### Requirement: Contacts past the far end
A striker's contact SHALL be measured where it lies, with no clamp; the damage routine SHALL ignore a contact at or past the striker's far end, a fist's included.

#### Scenario: A fist's far face
- **WHEN** a fist's contact lies on the far face of its hand's box
- **THEN** no wound is dealt
