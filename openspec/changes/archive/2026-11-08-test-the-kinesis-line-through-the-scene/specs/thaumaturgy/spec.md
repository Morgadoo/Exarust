## MODIFIED Requirements

### Requirement: Kinesis pick and aim
While the player's kinesis waits for its pick, a prop the hand could lift SHALL be in reach wherever it is when the kinesis sees the cursor's point, and SHALL be handed to the kinesis; any other press SHALL be out of reach. Each tick a cursor hit within 40 m SHALL push the held body toward the aim: the hit raised `100 − depth` (at least 10) back along the ray, kept when the line test from the head to it is clear, else the last aim. Without a hit only the keys SHALL move it.

#### Scenario: A lifted crate
- **WHEN** the player's kinesis runs and the hand lifts a crate
- **THEN** the kinesis holds the crate and aims at its centre, and the hook goes when the kinesis ends

#### Scenario: Only a prop is in reach
- **WHEN** the kinesis waits and the player presses a far crate it sees, then a near corpse
- **THEN** the kinesis holds the crate and the corpse press is refused as out of reach

#### Scenario: A crate the kinesis cannot see
- **WHEN** the kinesis waits and the line from the head to the pressed crate meets a wall
- **THEN** the press is out of reach and nothing is held

## ADDED Requirements

### Requirement: Kinesis sight and stamp
The kinesis SHALL see the cursor's point when the cursor's ray meets something with no active room's occluder before it and the line test from the caster's head to that point raised 5 cm, stopping 5 cm short, is clear. Every steering tick the held body SHALL be stamped with its caster until 2 s later.

#### Scenario: A hold longer than 2 s
- **WHEN** the kinesis has held a crate for 5 s and lets it go
- **THEN** the crate's stamp names the caster until 2 s after the last tick

### Requirement: Kinesis line test
The kinesis's line test between two points SHALL be blocked by an active room's occluder, then its lid, then a scene node on the segment: the level's meshes or a door that is not locked. Props, characters and locked doors SHALL let it through. A line of no length SHALL be blocked.

#### Scenario: A crate and a door on the line
- **WHEN** a crate, a locked door and an unlocked door stand on three lines from the head
- **THEN** the first two lines are clear and the third is blocked
