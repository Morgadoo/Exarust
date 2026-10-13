# pack-archives Specification

## Purpose

Reads the original `.rpk` table-of-contents archives, including archives
nested inside other archives, giving fast access to entries by name.

## Requirements

### Requirement: Archive table of contents
The system SHALL list every entry of a pack with its name, offset and size, and SHALL return each entry's bytes exactly as stored.

#### Scenario: Retail packs
- **WHEN** each of the 12 retail packs is opened
- **THEN** every entry is listed and its bytes lie within the file

#### Scenario: Sixteen-character names
- **WHEN** an entry name fills all 16 bytes with no terminator
- **THEN** the full 16-character name is returned

### Requirement: Case-insensitive lookup
The system SHALL find entries by name regardless of letter case.

#### Scenario: Mixed-case reference
- **WHEN** a model references texture `GrassPalA01D`
- **THEN** the entry stored as `grasspala01d` is found

### Requirement: Nested archives
The system SHALL read an archive held as an entry inside another archive with the same rules as a top-level pack.

#### Scenario: Material library
- **WHEN** `common.rml` is read from `Resource.rpk`
- **THEN** its material entries are listed by name

### Requirement: Malformed archives are rejected
The system SHALL report an error, without panicking, for a wrong magic, a truncated table, or an entry that points outside the data.

#### Scenario: Entry out of bounds
- **WHEN** an entry's offset plus size exceeds the archive length
- **THEN** opening fails with an error naming the entry
