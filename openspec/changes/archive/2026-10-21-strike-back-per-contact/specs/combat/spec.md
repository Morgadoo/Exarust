## ADDED Requirements

### Requirement: Turned parry
A parrying body whose state flag 0x400 is set SHALL take nothing from a clash (no lock, no attack-state clear, no sag) and SHALL make any striker recoil, whatever its size.

#### Scenario: Turned parry
- **WHEN** a striker of size 1.5 is parried by a body that turns blows
- **THEN** the parrier's swing state and stance are untouched and the striker recoils at strength 0.5, where a plain parry leaves that striker's swing

### Requirement: Striker per contact
The system SHALL deal each weapon contact's striker (its wound follow-through, cleave or recoil) right after that contact's struck side, before the next contact of the frame, as the original's damage routine runs once per contact.

#### Scenario: Two fighters strike each other
- **WHEN** two fighters wound each other in one frame
- **THEN** the first contact's striker takes its follow-through before the second contact stalls it as a struck attacker

### Requirement: Guard sounds once a swing
A clash and a block SHALL set the swing's weapon-sound bit 0x1000; a block SHALL set its shield-sound bit 0x200 once a swing and draw the shield's sound cooldown from the generator then (the wound bits already hold 0x200).

#### Scenario: Two blocks in one swing
- **WHEN** a swing is blocked twice
- **THEN** the generator steps once, and not at all for a swing that already wounded
