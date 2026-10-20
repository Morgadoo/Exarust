## ADDED Requirements

### Requirement: Fresh tree text
A fresh tree text SHALL carry language 0x6e65 (en), flags 0x4e4, kind 1, marker 0 and one empty string. The parser SHALL read the language, flags, kind and marker from the first 12 header bytes before the count.

#### Scenario: A new chunk
- **WHEN** a fresh tree text is created
- **THEN** its language is 0x6e65, flags 0x4e4, kind 1, marker 0 and its strings are one empty string
- **WHEN** a chunk with those header words is parsed
- **THEN** the parser returns the same four fields
