## ADDED Requirements

### Requirement: Engine fixed names and string compares
A sixteen-byte name SHALL be zero-padded; an ANSI text longer than sixteen bytes SHALL keep fifteen bytes and store 0x85 in the sixteenth. Byte compares SHALL answer the unsigned difference of the first differing pair; the ANSI compares SHALL answer the length difference when the common part is equal, the case-insensitive one comparing raised letters. The ANSI-string lowering SHALL lower every byte but the last.

#### Scenario: Ellipsis past sixteen
- **WHEN** the ANSI text `0123456789abcdefgh` becomes a name
- **THEN** its first fifteen bytes are `0123456789abcde` and the sixteenth is 0x85

#### Scenario: Case-insensitive ANSI compare
- **WHEN** `ABC` is compared with `ab` and `a` with `Abcd`
- **THEN** the answers are 1 and -3

#### Scenario: Lowering stops before the last byte
- **WHEN** `AbC\xE9 Z` is lowered in place
- **THEN** it reads `abc\xE9 Z`
