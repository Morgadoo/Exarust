# Spec Delta

## MODIFIED Requirements

### Requirement: Guardian sleep
Putting a guardian to sleep SHALL set the statue bit of its race word and its statue byte, with stance 1 and pose 2, clear its condition word, install the dormant behaviour and fade its glow material out; the first time, each limb SHALL take its stored mass times 0.05 and each part its value times 0.02, both in double precision, and the latch SHALL be set.

#### Scenario: A second sleep keeps the scales
- **WHEN** a guardian already latched is put to sleep
- **THEN** its limb and part scales are unchanged

### Requirement: Alarm return
While an alarm is armed, the alarm return SHALL offer the characters within 3000 cm of the player (x and z, strictly inside, AI-bit race words) in the player's room and the rooms behind its exits to the homeward callback, which SHALL refuse a guardian whose statue byte is set or whose condition carries the dying bit `0x800` and send any other home. The alarm SHALL stay armed.

#### Scenario: Every guardian asleep
- **WHEN** the alarm is armed and every guardian near the player is a statue
- **THEN** none goes home, the player stays where it fell and no draw is taken

#### Scenario: A dying guardian stays
- **WHEN** an awake guardian's condition carries `0x800`
- **THEN** the homeward callback refuses it

### Requirement: Alarm return in the story revive
The story revive of a knocked-out player (five seconds after its body died with red health of at least 0.01) SHALL run the alarm return after its prelude closes and before its save and the body's revive. A guardian sent home SHALL keep its pose about its root when it stands on its controller. With its statue byte set again, a guardian's sense SHALL clear its concern for the player, so it leaves combat.

#### Scenario: Golems sent home in play
- **WHEN** the player walks into the gate of pass 474 without its token, the two golems that wake fight it, and it falls with red health left
- **THEN** five seconds later both golems stand at their homes as statues at rest and the player lies 200 cm past the gate box's third-axis face

#### Scenario: The revive's order
- **WHEN** the story revive runs
- **THEN** the alarm return comes after the prelude closes and before the save and the body's revive
