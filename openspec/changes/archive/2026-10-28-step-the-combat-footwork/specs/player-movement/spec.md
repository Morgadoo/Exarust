## ADDED Requirements

### Requirement: Footwork step gate
In combat mode the footwork SHALL lay no step while the condition word has any of the bits 0x82C1, the energy is under 0.7 (a NaN energy lays none), a swing is under way, or the held movement word is zero.

#### Scenario: The gate
- **WHEN** W is pressed while the condition word is walking, or the energy is 0.69, or a swing is under way
- **THEN** no step starts

### Requirement: Footwork step under way
A footwork step SHALL be under way while it has not landed, or while its target is strictly more than 20·scale cm from the chest across the ground; a NaN distance SHALL not be under way. With no step under way the footwork SHALL lay the movement goal from the held word and start a step when the goal is taken.

#### Scenario: Started from rest
- **WHEN** W is pressed in combat mode on open ground with an 80 cm reach
- **THEN** a step starts toward a target 80 cm ahead

### Requirement: Young footwork step
Within 300 ms of a step's start, a word not yet a tap SHALL queue the raw word when the held word differs from the step's own word, the analog word queueing even when the same; a tapped word SHALL re-lay the step's goal at the tapped stride unless the step is already long.

#### Scenario: A held key re-lays the step
- **WHEN** W is pressed at 1000 ms and held through 1201 ms with an 80 cm reach
- **THEN** the step's target moves from 80 cm to 120 cm ahead

### Requirement: Footwork step queue
From 300 ms after a step's start any nonzero raw word SHALL queue the next step: the raw word and its full step vector, with a queue count of one. The queue SHALL clear once the word has been zero for more than 200 ms since its last nonzero tick.

#### Scenario: A held key queues
- **WHEN** W is held from 1000 ms to 1300 ms with an 80 cm reach
- **THEN** at 1300 ms the forward word is queued with an 80 cm step vector

### Requirement: Footwork step landing
The interim footwork SHALL move the capsule toward the step's target and land the step within 20·scale cm of it across the ground, or once it has run 1000 ms; a started step SHALL finish after the keys are released.

#### Scenario: A released step finishes
- **WHEN** W is tapped at 1000 ms and released at 1012 ms with the chest 30 cm short of the target, then 15 cm short
- **THEN** the player still steps at 30 cm short and stands in its stance once 15 cm short

### Requirement: Queued step taken on landing
A landing with a queued step SHALL take it by the body's queue rule: the target becomes the chest plus the queued vector, clamped by the line walk, and the queue count becomes −1 after the first landing. On the next tick the movement word SHALL take the queued word back, clear its tap flag and restart its change clocks.

#### Scenario: A landing takes the queue
- **WHEN** a queued forward step waits and the chest reaches the step's target
- **THEN** the new target is 80 cm further ahead, the queue count is −1, and on the next tick the word is forward with no tap

### Requirement: Combat mode word
Each combat tick the behaviour SHALL set its mode word: down or dazed (energy under 0.85, a NaN reading as enough) is idle and keeps the button word; switching held items or casting is idle and clears it; the right button walks; the left button's press attacks, a held left button keeps the mode and a swing makes it idle.

#### Scenario: Buttons
- **WHEN** the right button is held, then only the left button is pressed and held
- **THEN** the mode is the walk, then the attack, and stays the attack while held

### Requirement: Cursor walk by the mode word
The right button's walk to the cursor in combat mode SHALL run only in the combat mode word's walk mode.

#### Scenario: Dazed
- **WHEN** the right button is held in combat mode with the energy just under 0.85, then at 0.85
- **THEN** the player does not walk to the cursor, then it does

### Requirement: Footwork goal ignores the crouch
The movement goal SHALL lay its own stride from the step reach and SHALL NOT take the deep crouch's 0.7 share; a crouch past 0.5 SHALL leave the goal's target where an upright body's would be.

#### Scenario: A deep crouch
- **WHEN** W is pressed in combat mode with the crouch at 0.6 and an 80 cm reach
- **THEN** the target is 80 cm ahead

## MODIFIED Requirements

### Requirement: Footwork step targets
A footwork goal SHALL be chest + v·(r·extension·√scale/|v|) cm: v the key sum (W, A, D, S; a side key 0.8, height dropped), r the step reach or, for a tapped word, (1.5 − 0.3·encumbrance)·reach in double, narrowed once. The line walk SHALL clamp it from the chest's x and z at the ground; it SHALL be taken only when over 400 cm² from the chest in three dimensions, then moved onto the grid's nearest walkable point across the ground.

#### Scenario: Footwork at a wall
- **WHEN** W is held in combat mode facing a wall that clamps the step's target 20 cm ahead of the chest on its level, then 21 cm ahead
- **THEN** the player takes no step and plays no walk, then it steps and plays the combat walk

#### Scenario: A diagonal weighs the side by 0.8
- **WHEN** W and A are held together facing +Z, on open ground, for a fresh press of an 80 cm step
- **THEN** the target lies 0.8 as far to the side as it lies forward, and a side key alone still aims a full step

#### Scenario: A raised clamp
- **WHEN** the line walk clamps a step 15 cm ahead and 15 cm up, then 15 cm ahead on the level
- **THEN** the first starts a step and the second does not

#### Scenario: A held step is a long step
- **WHEN** W has been held past 200 ms with a step younger than 300 ms, for a human step reach and for a reach of 72 cm at encumbrance 0.5
- **THEN** the target is re-laid 120 cm and 97.2 cm along the keys

#### Scenario: A shortened extension halves the step
- **WHEN** the legs' extension is 0.5, for a plain and a tapped stride of an 80 cm reach
- **THEN** the strides are 40 cm and 60 cm

#### Scenario: A crouch of 0.1 keeps the short step
- **WHEN** W has been held past 200 ms with the crouch at 0.1, then at the float just under 0.1
- **THEN** the first target stays one step reach and the second is the long step

#### Scenario: The grid snap
- **WHEN** a taken goal lies 10 cm off the grid's nearest walkable point
- **THEN** the step's target is that point's x and z at the clamped height

### Requirement: Movement word persistence
The footwork's movement word SHALL be A 2, D 8, W 1 and S 4, A with D and W with S cancelling; a word pressed for less than 200 ms SHALL be kept until 200 ms after the press, and a word held longer SHALL clear on release. The analog stick SHALL replace the keys only when its squared length is over 0.01 in double precision, and a held word SHALL be flagged a tap after 200 ms only while the crouch is under 0.1. Exploration SHALL read the keys as they are held.

#### Scenario: A tap and opposing keys
- **WHEN** W is pressed at 1000 ms and released at 1010 ms, and later A and D are held together
- **THEN** the word stays forward until 1200 ms and is clear at 1201 ms, and A with D gives no word

## REMOVED Requirements

### Requirement: Deep-crouched step
**Reason**: The movement goal lays its own stride from the step reach; the behaviour's 0.7 share for a deep crouch is overwritten before the goal reads it.
**Migration**: See "Footwork goal ignores the crouch".


### Requirement: Long step latch
**Reason**: The long step is the movement word's tap flag read by the step dispatcher, not a latch of its own.
**Migration**: See "Footwork step dispatch" and "Movement word persistence".
