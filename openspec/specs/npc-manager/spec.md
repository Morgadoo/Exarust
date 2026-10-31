# npc-manager Specification

## Purpose
What the AI manager does with each character every frame, the trigger zones a moving character crosses, the group alert, and the clamp that keeps an approach out of the target's circle.

## Requirements

### Requirement: Manager frame
A dormant AI SHALL be ticked by the manager. A dead character in the scene whose rise flag is set and whose revival time has passed SHALL rise when its health pool B is at least 0.01, and SHALL lose the rise flag below that unless marked to keep it. A character that moved more than 3 cm since the last check SHALL check the trigger zones.

#### Scenario: Rising and checking
- **WHEN** a due undead has pool B 0.01, another 0.0099 unmarked, and a character moved 3 cm and then 3.001 cm
- **THEN** the first rises, the second stops rising, and only the 3.001 cm move checks the zones

### Requirement: Trigger zones
A zone SHALL be entered within its radius at a shape distance of at most 0.1 after one of 0.1 or more, at the character's frame anchor (root x and z, pelvis height). An entered `0x1xx` zone of kind `0x10` SHALL run its routine for anyone; for the player, kind 2 is the golem alarm and bit `0x20` the level exit (`0x21` flagged) or a host's release. A `0x2xx` zone of kind `0x10` SHALL run while the player is inside.

#### Scenario: Zone kinds
- **WHEN** a character enters a script zone, the player enters an alarm zone while held, and enters an exit zone while hosting
- **THEN** the routine runs, the held player ends the pass, and the host is released

### Requirement: Alert and approach
Raising the group alert SHALL add one and hold it 10 s. A step from outside a circle of 60 × the target's scale SHALL be cut where it crosses it when the original's test (the cut point's z offset and an unwritten 0) has a cosine of 0.75 or more, or none. Inside, a distance squared under the radius SHALL push out to the circle, else the step SHALL be dropped.

#### Scenario: A 60 cm circle
- **WHEN** a character 100 cm from a target of scale 1 steps 60 cm, 30 cm and diagonally past it, and characters 5 cm and 30 cm away step 30 cm in
- **THEN** the 60 cm step is cut to 40 cm, the others are kept, the 5 cm one is pushed out to 60 cm, and the 30 cm one is dropped

### Requirement: AI start-up
The AI start-up of a scene SHALL give the scene a `_Temp` node with flag bit 0x100000 when it has none, and SHALL independently start the AI manager only when none runs yet.

#### Scenario: A second scene
- **WHEN** a scene without `_Temp` starts while the manager already runs
- **THEN** the node is made and no second manager starts

### Requirement: Live script zones
Every frame, each character that moved more than 3 cm from its last checked point SHALL check the level's script zones in placement order, move its checked point, and run the routine of each zone it has just entered.

#### Scenario: Entered once
- **WHEN** a character steps from 2 m outside the checkpoint zone into it and then 5 cm further inside
- **THEN** the routine runs once
