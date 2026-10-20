## ADDED Requirements

### Requirement: Creation defaults
Opening SHALL clear 40 bytes at `0x198`, set the count to 2 and the stride to 16, and make two zeroed entries. The first entry SHALL get `0x101` and `0x5FFF` with `0x100000` ORed into the first word; the second SHALL get `0x201` and `0x5FFF`. The first and second entry words SHALL be stored, then 4.0 and `0x510000`.

#### Scenario: The opening
- **WHEN** the object opens
- **THEN** the count is 2 and the stride 16
- **AND** the first entry is `0x100101`, `0x5FFF`, 0, 0
- **AND** the second entry is `0x201`, `0x5FFF`, 0, 0
- **AND** the stored words are `0x100101` and `0x201`
- **AND** the value is 4.0 and the flags word is `0x510000`
