# Spec Delta

## MODIFIED Requirements

### Requirement: Mood tests
The mood of A toward B SHALL be a hostility code (0x600 at an engaging contact level, level bit 2; at the watching level 1, 0x600 with concern above 4, 0x200 above 1.2) and an attitude band (above 70, above 30, below −30, else: 0x10, 0x20, 0x80, 0x40), the attitude scaled by ((10 − x)/10)² for the contact's hostility x, zeroed above 10. An unordered value SHALL fail every compare. The request's mood code SHALL use the same bands in the top nibble after the watching and engaging codes.

#### Scenario: Unordered values
- **WHEN** the attitude is NaN, then a watched contact's concern is NaN
- **THEN** the mood is 0x40 with code 0x30000000, then no hostility code is set

#### Scenario: An engaged contact
- **WHEN** A's contact of B is at level 3 with hostility 4 and A's trust in B is −45
- **THEN** the mood is 0x640 (the attitude scaled to −16.2 is in the neutral band)

### Requirement: Script cast mirror
Each frame every live character SHALL reach the script cast with its name, roles, health as the red pool over the maximum, arousal (the brain's arousal word, which a new brain starts at 10 and its contact integration drifts; 0 without a brain) and the collapsing/dead state bit, keeping its relations, topic memory and role values; a role list a script changed SHALL be written back to the AI's roles first and its change flag cleared.

#### Scenario: A script adds a role
- **WHEN** a character holds role 5 and a script adds role 7 in front of it
- **THEN** after the next frames both the AI and the cast hold roles 7 then 5

#### Scenario: Script health
- **WHEN** a body has red health 0.2, yellow 0.1 and maximum 0.25
- **THEN** the cast health is (0.2, 0.25), so `Health` gives 80

## ADDED Requirements

### Requirement: AI contact mirror
Before and after each fixed step every brain's character contacts SHALL reach its cast character with the other character, the concern, the trust toward the other (the relation's last attitude, 0 without one), the classification level, the hostility, and the noticed level and attention offset as the hearing. Object contacts and contacts whose other has no character id SHALL be left out.

#### Scenario: An engaged contact reaches the cast
- **WHEN** a brain holds a contact at level 3, concern 2.5, hostility 4, noticed 1.5 and attention 0.25, and its relation's last attitude is −45
- **THEN** the cast contact carries those values with trust −45

### Requirement: Contact write-back
A concern, noticed level or attention offset a script or a line changed since the last mirror SHALL be written back to the brain's contact before the next mirror; a value the AI writes later SHALL reach the cast.

#### Scenario: A script raises a concern
- **WHEN** a script sets a contact's concern to 12 and a line is heard at 7.5
- **THEN** the brain's contact holds concern 12, noticed level 7.5 and attention 0 after the next sync

### Requirement: Player contacts
The player's cast contacts SHALL be its behaviour's contact list, the other characters only.

#### Scenario: One character near the player
- **WHEN** the player's behaviour lists one character
- **THEN** the player's cast character has that one contact, its other values zero

### Requirement: AI greetings
The greeting a target pass reports (a newly attended character within 6 m, with the brain's notice scale at most 6 and its arousal at most 1) SHALL run after that pass's encounter scripts: the brain's contacts SHALL be synced into the cast, then the dialogue engine SHALL look for a calm class 0 entry node of the pair with the brain's mood code; a brain whose arousal is above 1 or already in a conversation SHALL not greet.

#### Scenario: The mood picks the branch
- **WHEN** a guard greets a contact it watches with concern 5, its greeting holding a calm-only branch before an unflagged one
- **THEN** the unflagged branch speaks; with a calm contact, the calm branch speaks

### Requirement: Memory witness event
A memory fragment's story event SHALL run the witness event from the player: each of the player's contacts holding the fragment's role SHALL have the fragment's topic of its dialogue toward the Player role activated in its memory of the player.

#### Scenario: Derrin witnesses a memory
- **WHEN** fragment 4 arrives for a player holding the memory role whose contact holds XA_Derrin, with an `XAU_Memory` node in its dialogue toward the Player role
- **THEN** that contact's memory of the player under (Player, XA_Derrin) holds the topic activated

### Requirement: Script reports
A script's report of an invalid role identifier SHALL be logged as the engine's level-4 message, `title : text`.

#### Scenario: An invalid role set after a branch
- **WHEN** a branch runs for an actor holding a role the database lacks
- **THEN** the log carries `Process Event : Actor 0 invalid role identifier.`
