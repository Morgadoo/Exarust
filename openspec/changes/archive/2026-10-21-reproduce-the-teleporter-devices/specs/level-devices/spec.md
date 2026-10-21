## ADDED Requirements

### Requirement: Device crystal
The system SHALL accept on a teleporter's slot only an object whose whole record id is Power Crystal A (431) or the large-crystal adapter (1423) and whose condition is below 0x55, and on a reconstruction device's slot only a Power Crystal L (500) whatever its condition.

#### Scenario: A tier bit
- **WHEN** the object on a teleporter's slot has the record id 0x800001AF
- **THEN** the use is refused

#### Scenario: A spent large crystal
- **WHEN** a Power Crystal L of condition 255 lies on a reconstruction device's slot
- **THEN** the scan starts

### Requirement: Device reach
The system SHALL refuse a device use when the squared distance from the point 16 cm above the device to the player is more than 10 000 cm², and SHALL let a NaN distance pass.

#### Scenario: The edge
- **WHEN** the player stands 96 cm along x and 12 cm below a device's origin
- **THEN** the use is in reach

### Requirement: Destination rows
The system SHALL lay out a teleporter screen's rows by walking its first four destination ids (one with flag bit 2) in order, sending an empty id, or one met when the other rows are taken, to row 0 and any other id to the next free row, without stopping at an empty id.

#### Scenario: The shipped teleporter
- **WHEN** the destinations are 7, 10, 12 and 13
- **THEN** the rows show 13, 7, 10 and 12

#### Scenario: An empty entry
- **WHEN** the destinations are 7, 0, 12 and 13
- **THEN** the rows show the entries 1, 0, 2 and 3

### Requirement: Device scan
The system SHALL start a reconstruction-device scan as a scan effect on the player that waits 1600 ms of game time and loops its close with the flash, SHALL play `devicescna01` at 0.3 at the player, and SHALL start nothing while a scan effect is already on the player.

#### Scenario: A second use
- **WHEN** the device is used again while its scan is on the player
- **THEN** the player still carries one scan effect

### Requirement: Device glow
The system SHALL scale a used reconstruction device's glow by a factor that rises over 1500 ms, holds at one for 3800 ms, falls over 1500 ms and ends at zero.

#### Scenario: On the fall
- **WHEN** 6050 ms have passed since the use
- **THEN** the factor is 0.5

#### Scenario: Over
- **WHEN** 6801 ms have passed
- **THEN** the glow is over and the factor is 0

### Requirement: Reconstruction
The system SHALL end a story session whose player's red health is below 0.01 (compared in double precision) in a reconstruction when a reconstruction device scanned the same record identity, stripping the player only on the first such death, and in an ordinary death otherwise.

#### Scenario: The first death after a scan
- **WHEN** the scanned player dies in the story
- **THEN** the player is reconstructed with no held or worn items and just under half health

#### Scenario: A later death
- **WHEN** the reconstructed player dies again
- **THEN** the player is reconstructed without a strip

#### Scenario: Red at 0.01 in single precision
- **WHEN** the player's red health is the single-precision 0.01
- **THEN** the player is dead

### Requirement: Left-behind record
The system SHALL leave, for an unshared record, a copy of the record before the strip as the corpse, with all its items, one use, the 0x40 mark, identity 0, roles 1 and 0xC2 and no health or stamina.

#### Scenario: A dressed corpse
- **WHEN** a player holding four items and wearing two is stripped
- **THEN** the left-behind copy holds the four items and wears the two

### Requirement: Device power-down
The system SHALL play `devicepwra01` at 0.1 at a reconstruction device and clear its latch when an object is let go within a squared 5 cm of its slot point.

#### Scenario: The crystal taken off
- **WHEN** the crystal is lifted off a scanned device's slot
- **THEN** a later death in the story is an ordinary death
