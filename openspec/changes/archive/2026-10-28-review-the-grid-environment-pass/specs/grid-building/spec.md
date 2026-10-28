## ADDED Requirements

### Requirement: Special tile floor
The system SHALL keep a special tile's centre floor only for types 6 to 9 and SHALL turn every one of the ten special masks, the tenth included, by the cell's rotation.

#### Scenario: Floor by type
- **WHEN** cells carry special tile types 1 to 10
- **THEN** only types 6, 7, 8 and 9 place their centre floor

#### Scenario: Tenth type turned
- **WHEN** a cell's special tile of type 10 has rotation 1
- **THEN** its mask is 00770FFF turned one quarter, not zero

### Requirement: Set catalog
The system SHALL register each wall set and floor set name once per kind, comparing whole names, and SHALL mark a floor set named exactly `Terrain` with the flags 0x7E01 in the load pass instead of loading it.

#### Scenario: Shared name
- **WHEN** two cells name GEWS_UWA01 and a third names GEWS_SWA01
- **THEN** the wall catalog holds two entries and GEWS_UWA01 keeps index 0
