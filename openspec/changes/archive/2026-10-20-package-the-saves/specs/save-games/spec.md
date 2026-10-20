## ADDED Requirements

### Requirement: Save package
The system SHALL read and write the retail package: the magic, the sentinel and 24-byte directory, the 16-byte section names and the 0x2000 alignment.

#### Scenario: Two sections
- **WHEN** a package is written with `arenahub` and `exanima04`
- **THEN** the entries start at 0x2000 and 0x4000 and the total is 0x4009
- **WHEN** the package is parsed
- **THEN** both sections are returned by their exact names

### Requirement: Save chunks
The system SHALL frame payloads as `u32 id, u32 len, payload`.

#### Scenario: One chunk
- **WHEN** chunk 0x100000 with four bytes is written and parsed
- **THEN** its id and payload round-trip

### Requirement: Player-data headers
The system SHALL parse the 48-byte story (`0x6F5AEA14`) and arena (`0xE8AAEA10`) headers.

#### Scenario: Story and arena
- **WHEN** a story header names exanima04
- **THEN** the flavour is Story and the level is exanima04
- **WHEN** an arena header has field36 9
- **THEN** the flavour is Arena and field36 is 9
