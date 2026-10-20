## ADDED Requirements

### Requirement: Arc colour and parts
The system SHALL use the spark colour (0, 53, 0), treat kind 1 below 0xfe or kind 0x11 below 0x7f as live, and kind 4 below 0xfe as an arc-spark part.

#### Scenario: Live and spark
- **WHEN** the kind word is 1 and the byte 0xfd
- **THEN** the part is live
- **WHEN** the kind word is 4 and the byte 0
- **THEN** the part is a spark part

### Requirement: Spark brightness
The system SHALL compute the random brightness as 0.8 + 0.2·roll.

#### Scenario: Two rolls
- **WHEN** the roll is 0
- **THEN** the brightness is 0.8
- **WHEN** the roll is 0.5
- **THEN** the brightness is 0.9

### Requirement: Released electrified effect
The system SHALL hide a 0x21-kind node, set its state to 1 and clear its callbacks on release; the callbacks SHALL clear whether or not the node matched.

#### Scenario: Released and other
- **WHEN** a 0x21-kind node with visible 1 is released
- **THEN** visible is 0, state 1 and callbacks are gone
- **WHEN** a 0x22-kind node is released
- **THEN** its visible byte is unchanged but its callbacks are gone
