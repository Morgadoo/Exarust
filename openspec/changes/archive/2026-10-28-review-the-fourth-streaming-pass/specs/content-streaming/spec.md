## MODIFIED Requirements

### Requirement: Placed object flags
A placed object's object flags and its placement's static bit SHALL be kept; they mark only the object's root node, whose bits 0xB80000 SHALL be replaced by the object flags' same bits and which SHALL gain 0x80000 when the placement word has 0x10, and the object's bodies SHALL still register (or stay with their mechanism).

#### Scenario: A placed object marked static
- **WHEN** a placed object's object flags carry 0x80000
- **THEN** it reports static and its bodies are registered as before

#### Scenario: The root node word
- **WHEN** a root node word of 0xFFFFFFFF takes object flags 0
- **THEN** it reads 0xFF47FFFF

## ADDED Requirements

### Requirement: Character size words
A character's two stored size words SHALL each scale the body's base scale by the word times 1/65535 in double precision, narrowed to single precision last, the second word's field written first. Only the streamed builder of a body that is not dead SHALL replace a pair whose second word is zero with 0xFFFF and 0xFFFF; the request builder and a dead body SHALL scale the words as stored.

#### Scenario: Half a word
- **WHEN** a word of 32768 scales a base of 1
- **THEN** the scale is 0.5000076

#### Scenario: An unset pair
- **WHEN** a living streamed body has the pair 0x8000, 0
- **THEN** both words become 0xFFFF and both scales equal the base
- **AND** a dead body keeps 0x8000, 0

### Requirement: Character request flags
A character request's bit 0x2000 SHALL add 0x1000 to the built character's node word, and its bit 0x1000 SHALL skip both starting the character's AI and restoring its saved AI pose (three floats and a fourth word).

#### Scenario: A marked request
- **WHEN** a request with 0x2000 builds a node whose word is 0x40
- **THEN** the node word is 0x1040

### Requirement: Saved character chunks
A streamed character's saved blob SHALL be a 32-bit length of the bytes that follow it and then tag, size and body records, scanned from byte 4 while the record lies within the stored length, keeping the last body of each of the flat pose, motion, pose tree and AI home tags; a malformed record SHALL end the scan.

#### Scenario: A repeated tag
- **WHEN** a blob holds two motion records, of one and of two bytes
- **THEN** the motion chunk is the two-byte body
