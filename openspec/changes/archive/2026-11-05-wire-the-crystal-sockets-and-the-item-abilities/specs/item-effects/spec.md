# Spec Delta

## RENAMED Requirements

- FROM: `### Requirement: Item-effect glow`
- TO: `### Requirement: Self-sharpening slash share`

- FROM: `### Requirement: Hand-effect dispatch`
- TO: `### Requirement: Held-item ability dispatch`

## MODIFIED Requirements

### Requirement: Self-sharpening slash share
The system SHALL compute a weapon's slash share as 0.5·(0.01·slash)^0.7 in double precision. A hit struck with a self-sharpening blade SHALL first blunt its slash byte: one above 0x40 loses four, any other becomes 0x3c.

#### Scenario: Floor and first step
- **WHEN** the slash byte is 0x3c or 0x40 when the blade strikes
- **THEN** the byte becomes 0x3c and the share is 0.5·0.6^0.7
- **WHEN** the slash byte is 0x44
- **THEN** the byte becomes 0x40 and the share is 0.5·0.64^0.7

### Requirement: Held-item ability dispatch
An item in either hand SHALL attach the lightning check, the self-sharpening check or the burning check when its weapon section's ability word is 1, 2 or 3, and nothing for any other word (the sceptre's 4 included); the self-sharpening blade SHALL also get its hit hook.

#### Scenario: A masked word does not count
- **WHEN** the ability word is 0x22
- **THEN** nothing is attached

## ADDED Requirements

### Requirement: Self-sharpening check
Once a game second each character's ability list SHALL run and count the run. On a run whose count has its low three bits all set, a held self-sharpening blade whose first socket holds a Power Crystal B (low five bits 2) with a charge below 0xff and whose slash byte is below 0x8c SHALL gain one in the charge and two in the slash byte, and on that run the record's slash share SHALL be rewritten from the slash byte either way.

#### Scenario: Sixteen runs
- **WHEN** a blade with slash 80 and crystal B at charge 0 is held for runs 1 to 16
- **THEN** it sharpens on runs 7 and 15, to slash 84 and charge 2

#### Scenario: Shipped sword
- **WHEN** `exarust grid "play arena --ability-item 2 --opponent --still-opponent …"` runs
- **THEN** the log shows "Swrd Self Sharp" sharpening from slash 100 on run 7, and each landed hit blunting it by four

### Requirement: Lightning mace
A held item of ability 1 SHALL raise its hum arcs on a node 53 cm along its +Y when its first socket holds crystal A (low five bits 1) with a charge below 0xfe, or crystal AR (0x11) with a charge below 0x7f, and none are live: twenty respawning tendrils of five points at size 2, the segment shortened to 0.75, light 24 with the record's voxel light, the `electricity03` hum at 0.075. A spent socket SHALL end live arcs.

#### Scenario: Shipped mace
- **WHEN** `exarust grid "play arena --ability-item 1 …"` runs
- **THEN** "UB5 FlangedMace" raises its arcs on the first check

### Requirement: Lightning hit
While the mace's arcs are up, a hit struck with it SHALL shock the struck character with zero strength, add 10 to the socket's charge (0x80 for a kind with bit 0x10) while the sum stays below 0xff, else set it to 0xff and end the arcs, and leave a spark burst at the contact.

#### Scenario: Charging the socket
- **WHEN** a crystal A socket at charge 0xf4 and one at 0xf5 are struck with
- **THEN** the first becomes 0xfe and the second 0xff with its arcs ended

#### Scenario: AR crystal
- **WHEN** an AR socket at charge 0 is struck with
- **THEN** its charge becomes 0x80 and the next check ends the arcs

### Requirement: Every character's abilities
The ability checks SHALL run for every character holding an item with an ability, not only the player.

#### Scenario: A duellist's lit blade
- **WHEN** `exarust grid "play exanima01 --ability-item 3,npc-only --opponent --still-opponent --no-torch"` runs
- **THEN** the duellist's "Sword GSF 01" lights and the player's does not
