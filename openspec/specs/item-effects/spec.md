# item-effects Specification

## Purpose
Tracks the character's active item effects: each held or worn item's tick,
its charge, and the glow it drives.

## Requirements

### Requirement: Self-sharpening slash share
The system SHALL compute a weapon's slash share as 0.5·(0.01·slash)^0.7 in double precision. A hit struck with a self-sharpening blade SHALL first blunt its slash byte: one above 0x40 loses four, any other becomes 0x3c.

#### Scenario: Floor and first step
- **WHEN** the slash byte is 0x3c or 0x40 when the blade strikes
- **THEN** the byte becomes 0x3c and the share is 0.5·0.6^0.7
- **WHEN** the slash byte is 0x44
- **THEN** the byte becomes 0x40 and the share is 0.5·0.64^0.7

### Requirement: Item-effect charge
The system SHALL accumulate charge as min(current + source·0.004, source).

#### Scenario: Clamped charge
- **WHEN** the current is 99.9 and the source is 100
- **THEN** the charge is 100
- **WHEN** the current is 0 and the source is 100
- **THEN** the charge is 0.4

### Requirement: Item-effect entries
The system SHALL give a worn part a glow weight of 1 only for an odd kind below 0xff, and SHALL attach a keyed entry once.

#### Scenario: Glow and attach
- **WHEN** the kind is 1 and the byte 0
- **THEN** the glow weight is 1
- **WHEN** an entry with an existing key is attached
- **THEN** the list is unchanged

### Requirement: Item charge tick
A held item's charge tick SHALL run while its record kind's low five bits are 2 and its counter byte is below 0xff: the body's accumulator gains the source times 0.004 in double precision; while the sum is at or under the source the counter byte advances and the player's HUD is shown for 1000 ms, and once it passes the source it is held at the source and nothing else changes.

#### Scenario: Under and over the source
- **WHEN** the player's accumulator is 0 with a source of 100
- **THEN** it becomes 0.4, the HUD is shown and the byte advances
- **WHEN** it is 99.9 with a source of 100
- **THEN** it becomes 100 and the byte does not advance

### Requirement: Held-item ability dispatch
An item in either hand SHALL attach the lightning check, the self-sharpening check or the burning check when its weapon section's ability word is 1, 2 or 3, and nothing for any other word (the sceptre's 4 included); the self-sharpening blade SHALL also get its hit hook.

#### Scenario: A masked word does not count
- **WHEN** the ability word is 0x22
- **THEN** nothing is attached

### Requirement: Garment steps
Only an odd item kind SHALL step its counter byte: by one, or ten with the fast bit, per counter above one, set to 0xff when the step would reach it. A worn item's light SHALL turn on while the byte is still rising and off once it is not; an even kind is never rising.

#### Scenario: An even kind
- **WHEN** a lit worn item of kind 0x12 is ticked with both counters above one
- **THEN** its byte is unchanged and its light turns off

### Requirement: Material fade
The material fade SHALL write only the first material whose parameter word matches: the square of the value, and the material marked visible.

#### Scenario: Two matching materials
- **WHEN** two materials carry the parameter word and the value is 0.5
- **THEN** the first fades to 0.25 and the second is unchanged

### Requirement: Worn effects in play
A spawned body SHALL get the effect list its worn items register: a healing item only when the body takes remedies, a golem suit's and gauntlets' ticks on any body, each item once.

#### Scenario: No remedies
- **WHEN** a body whose rules do not take remedies wears only a healing item
- **THEN** it gets no effect list

### Requirement: Effect list once a second
The effect list SHALL run when the game clock is past the stored clock, which then becomes the clock plus 1000 ms; an empty list SHALL clear the two impact counters instead.

#### Scenario: Once a second
- **WHEN** a body's list runs at 12 ms
- **THEN** the next run is at 1012 ms; nothing runs at 1012 ms and it runs again at 1013 ms

### Requirement: Worn healing
A healing item whose flag byte's low five bits are 2 and whose charge is at most 0xFE SHALL add 0.004 of the maximum to red health in double precision, holding red at the maximum when it passes it and otherwise advancing the charge.

#### Scenario: Healing step
- **WHEN** red is 0.1 of 0.25 at a run
- **THEN** red becomes 0.1 + 0.001 and the charge advances by one

#### Scenario: Held at the maximum
- **WHEN** red is 0.2495 of 0.25 at a run
- **THEN** red becomes 0.25 and the charge does not advance

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
