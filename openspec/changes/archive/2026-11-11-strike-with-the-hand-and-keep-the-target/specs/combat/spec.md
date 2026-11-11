## ADDED Requirements

### Requirement: Fists strike by touch
An unarmed fighter's fist on a part of its body SHALL strike through that hand part's pair contacts, as the striking part classifies a held weapon body's: a muscled limb of another fighter is a wound at the contact point, a held shield or torch a block, a held weapon a clash. Every contact faces the striking side. Such a fist SHALL take no blade sweep.

#### Scenario: Punch through contact
- **WHEN** an unarmed fighter's right hand, its window open, meets another fighter's chest in the pair stage
- **THEN** the fist's hit is a wound on the chest's bone at the contact point, with the fist as the striker

#### Scenario: Window shut
- **WHEN** the same hand meets the chest with its window shut
- **THEN** no hit is sent

### Requirement: Fist strike window bit
While a fist's slot window is open on an unarmed fighter, that fist's hand part SHALL carry the strike-chain bit, written only when the window opens or shuts; the bit SHALL be cleared when the fist leaves the world.

#### Scenario: Open and shut
- **WHEN** the first slot's window opens, then shuts
- **THEN** only the right hand part takes the bit, and loses it again

### Requirement: Struck fist clash
A striker that meets another fighter's fist hand SHALL clash with that fist, as the hand carries the weapon bit: the struck item is the fist, primary when it is its holder's primary striker.

#### Scenario: Weapon on a fist
- **WHEN** a sword's window is open and its body meets an unarmed fighter's right hand
- **THEN** the hit is a clash on that fighter's right fist

### Requirement: Combat target
Each fighter SHALL keep its combat target, written in its tick's attack stage: the player's behaviour auto-target, else its brain's target pick, else a brainless duellist's foe while it is in combat with a character foe; none otherwise.

#### Scenario: Out of combat
- **WHEN** a brainless duellist has not entered combat
- **THEN** its combat target is none

### Requirement: Combat target for aim and parry
The attack starters' aimed pick and the parry check SHALL read the fighter's combat target: a target in combat for the aim, an attacking target passing the trigger for the parry; no target gives neither. A fighter with no kept target SHALL take the nearest fighter in combat.

#### Scenario: Target is the foe
- **WHEN** a fighter whose target is its attacking foe is checked
- **THEN** it keeps the foe's parry for its starters and parries

#### Scenario: No target
- **WHEN** the same fighter has no target
- **THEN** it keeps no parry for its starters and does not parry
