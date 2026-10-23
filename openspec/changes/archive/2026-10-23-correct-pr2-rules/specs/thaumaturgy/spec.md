## MODIFIED Requirements

### Requirement: Kinesis steering
Kinesis SHALL steer its held body at its reach (half the number of parts whose parent is the held part, plus one) along the unit direction to the aim, or the direction times 0.02 within 50 cm or for a distance that is not a number, and SHALL turn it by `reach²` times the double 0.001 a tick, each key pair tested if/else.

#### Scenario: Both turn keys
- **WHEN** a held body with 100 parts linked to its held part has all four turn keys held
- **THEN** it turns forward on both axes by 2601 times the double 0.001, narrowed

### Requirement: Kinesis pick and aim
While the player's kinesis waits for its pick, a prop the hand could lift SHALL be in reach wherever it is and SHALL be handed to the kinesis, and any other press SHALL be out of reach. Each tick a cursor hit within 40 m SHALL push the held body toward the aim: the hit raised `100 − depth` (at least 10) back along the ray, kept when the head's line to it is clear, else the last aim. Without a hit the body SHALL NOT be pushed, but the keys SHALL still turn it.

#### Scenario: A lifted crate
- **WHEN** the player's kinesis runs and the hand lifts a crate
- **THEN** the kinesis holds the crate and aims at its centre, and the hook goes when the kinesis ends

#### Scenario: Only a prop is in reach
- **WHEN** the kinesis waits and the player presses a far crate, then a near corpse
- **THEN** the kinesis holds the crate and the corpse press is refused as out of reach

### Requirement: Throw body
A throw SHALL take the body the caster's current kinesis cast holds, and SHALL be refused when that kinesis holds none; without a kinesis cast it SHALL take the first body under the cursor past the level: a dead character, or an unheld prop whose mobility in double is at least 0.01. Each tick its velocity SHALL move the body's first part by its origin weights, less a fifth of its motion while steered, and wake it; a body gone SHALL end the throw.

#### Scenario: A kinesis crate thrown
- **WHEN** a throw is cast while the player's kinesis holds a crate
- **THEN** the kinesis ends and the throw holds the crate, aiming at its centre and spending nothing a tick

#### Scenario: An empty kinesis
- **WHEN** a throw is cast while the player's kinesis holds nothing
- **THEN** the throw is refused

### Requirement: A character's caster
A duellist's or placed character's caster SHALL take its body's technique word from the record's learnt techniques, each perk at the bit the original assigns it (Sustain 0x10_0000, Clarity 0x20_0000, Abstraction 0x40_0000, Iteration 0x80_0000, Volition 0x1_0000), and its persistent slots from Concentration's points and techniques by the slot count. A record with no known power SHALL make no caster.

#### Scenario: Concentration's techniques
- **WHEN** a record with powers has 15000 Concentration points and Sustain, Continuity II and Abstraction
- **THEN** its caster's technique word is 0x50_0000 and it has three persistent slots with the focus item, and with 5000 points none

### Requirement: A cast blast
A blast or bolt cast SHALL make a blast entity whose start plays cast motion 0x22 with the setup's power id and, when that motion starts, sets the caster's cast hook to answer -1. While it charges, any call of that hook SHALL fizzle it, and it SHALL pay its drain each tick whether or not it holds the hook.

#### Scenario: A duellist's blast
- **WHEN** a duellist with focus casts the blast at a body whose limb sits in front of its hand, while its body still reads casting
- **THEN** it charges on the hook for 101 ticks, is thrown with the hook and power id cleared, moves that limb and leaves the scene once its chunks are spent

#### Scenario: A later cast fizzles it
- **WHEN** another cast starts while the blast charges
- **THEN** the blast fizzles

### Requirement: A cast bubble
A bubble cast SHALL make a dome entity of radius 120 at the cast place whose start plays cast motion 0x24 with power id 0x20207 and takes a hook answering 0 to events with bits 0x28, refusing a cast's start (0x20). Standing SHALL clear the hook and power id, leaving the cast motion playing; the power's ending, or its caster being gone, SHALL break the dome once, and it shrinks and leaves. It SHALL hold its caster's parts in and push other bodies' covered parts out.

#### Scenario: A duellist's bubble
- **WHEN** a duellist with focus casts the bubble with a body's limb inside its reach, then casts it again once it stands
- **THEN** the dome grows on the hook, moves that limb, stands with the hook cleared, then shrinks and leaves the scene

#### Scenario: The caster is gone
- **WHEN** a bubble's caster is removed
- **THEN** the dome breaks once, shrinks over 700 ms and leaves the scene

### Requirement: A cast wave
A wave cast SHALL make a wave entity round the caster's body root whose start plays cast motion 0x24 with power id 0x204 and sets the blast's hook. While it charges it SHALL pay the cost times the tick's seconds; any call of its hook SHALL fizzle it, and a shortfall SHALL end the cast through the cast end, fizzling the wave only while it holds the hook.

#### Scenario: A duellist's wave
- **WHEN** a duellist with focus casts the wave at a body whose limb sits in front of its hand
- **THEN** it charges on the hook, is thrown with the hook and power id cleared, moves that limb and leaves the scene once spent

#### Scenario: A wave runs short
- **WHEN** a charging wave that holds the hook cannot pay its tick
- **THEN** it fizzles, the cast ends and the caster returns to its stance

### Requirement: Cast hooks answer by event
A cast hook SHALL answer 0 to an event carrying any of its kept bits and -1 to any other, except that a hook keeping every bit SHALL answer 0 to every event and the vanish's hook from its hold on SHALL answer -1 to any event with a 0x120 bit and 0 to any other. A cast SHALL NOT start while the running hook answers 0x20 with 0 or more.

#### Scenario: A shell's raise
- **WHEN** a shell starts its raise and another cast asks to start before the Force Raise clip ends
- **THEN** that cast is refused, and once the clip ends into the stance the hook and power id are dropped and casts start again

#### Scenario: Strikes take their hook with the motion
- **WHEN** Force Strikes or a barricade starts its cast motion
- **THEN** its hook refuses another cast until the clip ends, and a strikes cast whose motion is refused makes no strikes

#### Scenario: A kinesis hands its body to a throw
- **WHEN** a throw is cast while a kinesis holds a body
- **THEN** the kinesis ends without ending the cast, its hook refuses other casts, and the throw plays on in the kinesis's motion

#### Scenario: A vanish's hook lets a cast through once it holds
- **WHEN** another cast asks to start while a vanish runs
- **THEN** it is refused before the vanish's hold and allowed from the hold on, and either way the vanish is let go

### Requirement: The player's buttons and the release ask the hook
Each tick a living player's controller SHALL ask the hook with 1 while the left button is down, with 8 outside combat mode, and in combat mode, with the body casting and its tone not under 0.85, with 2 while the right button is down or the stick's length² passes 0.1 with Sprint held, else 1 for the left. A negative answer SHALL end the cast, and a casting body SHALL ask no attack.

#### Scenario: Strikes held through the buttons
- **WHEN** a strikes hook is asked by the left button out of combat mode, then by both buttons in combat mode while casting
- **THEN** it keeps the cast for 1 and 8, and the right button's 2 ends it

#### Scenario: A shock outside combat mode
- **WHEN** the player leaves combat mode while a shock charges
- **THEN** the controller's 8 ends the cast and the charge clears

#### Scenario: A weak caster
- **WHEN** the player, casting in combat mode at tone 0.8, holds the right button
- **THEN** the hook is not asked with 2

## ADDED Requirements

### Requirement: Blast shortfall and throw
A charging blast whose focus falls short while a cast hook is set SHALL end the cast through the cast end, fizzling the blast only while that hook is its own. The blast's throw SHALL clear the hook and the power id itself, leaving the cast motion playing, and a thrown blast SHALL push the limbs its chunks reach.

#### Scenario: A refused motion's blast
- **WHEN** a blast whose cast motion was refused, so no hook is set, charges with too little focus
- **THEN** it pays its drain and keeps charging, and the cast is not ended

#### Scenario: The throw keeps the motion
- **WHEN** a charged blast is thrown
- **THEN** the hook and power id are clear, the condition still reads casting and the cast motion plays on

### Requirement: Wave throw
A wave's throw SHALL clear the hook and the power id itself, leaving the cast motion playing, and a thrown wave SHALL push the limbs its chunks reach.

#### Scenario: A paid wave thrown
- **WHEN** a paid wave is released
- **THEN** it is thrown with the hook and power id cleared and the cast motion still playing

### Requirement: The cast end
The cast end SHALL clear the hook, the hook owner and the power id and, when the condition word is exactly casting (0x20), set it to 0 and return the body to its stance, blended over 30 frames, in the same tick. A cast motion started before that stance return plays SHALL drop it. A live effect's focus shortfall, kinesis, throw and the power drain SHALL end a cast through it.

#### Scenario: A running torrent runs short
- **WHEN** a looping torrent's focus falls short while it runs
- **THEN** the condition becomes 0, the hook and power id are clear and the stance starts with a 30-frame blend in that tick

#### Scenario: A new cast before the stance
- **WHEN** a cast motion starts after a cast end and before its stance return plays
- **THEN** the stance return is dropped and the new cast motion plays

### Requirement: A replaced motion asks the hook
When a motion other than the walk, the run or an attack is replaced, including a one-shot cast clip ending into the stance, the hook SHALL be asked with event 4 and a negative answer SHALL drop it and the power id. A looping cast clip SHALL loop.

#### Scenario: A one-shot clip ends
- **WHEN** a shell's Force Raise clip ends into the stance with its hook set
- **THEN** the hook is asked with 4 and, answering −1, is dropped with the power id

### Requirement: Vanish let-go
A running vanish SHALL be let go by any hook call carrying a bit of 0x1b1, whatever it answers.

#### Scenario: Events once it holds
- **WHEN** a vanish that holds is asked with 0x110, then 0x80, then 4
- **THEN** 0x110 answers −1 and lets it go, 0x80 answers 0 and lets it go, and 4 answers 0 and keeps it

### Requirement: A later cast is a new hook
A live effect SHALL take its hook as called once a later cast has started, even when that cast has the same power id and hook.

#### Scenario: A blast recast while charging
- **WHEN** the blast is cast again while the first blast charges
- **THEN** the first blast fizzles

### Requirement: The release flag asks the hook
With a cast motion playing, a set release flag SHALL ask the hook with 0x10, end the cast whatever it answers, and clear.

#### Scenario: A shock released
- **WHEN** the release flag is set while a shock charges in its cast motion
- **THEN** the hook is asked with 0x10, the cast ends and the charge clears

### Requirement: The player's caster
The player's caster SHALL keep technique word 0 and one persistent slot. The slot count SHALL read only the record's six technique bytes, however far Concentration passes 30000.

#### Scenario: The player and a high Concentration
- **WHEN** the player's caster is made, and a duellist's record has 35000 Concentration points
- **THEN** the player's technique word is 0 with one slot, and the duellist's slots come from its six technique bytes alone

### Requirement: A hidden duellist casts nothing
A duellist out of the scene SHALL run its combat state, but the game SHALL NOT answer its slots, cast its chosen power or let its cast go until it is back in the scene (Exarust's own).

#### Scenario: A duellist out of range
- **WHEN** a duellist that knows the shock is out of the scene with a target 3 m ahead
- **THEN** its slots are not answered and no shock starts
