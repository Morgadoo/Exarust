# data-tail Specification

## Purpose
The small helpers of the original's data layer: the sorted id index, locale lookup, named-block memory archive, project path rules, pack open rules and the debug command registry.

## Requirements

### Requirement: Sorted id index
The index SHALL keep its entries in increasing id order, store each block with a first word holding the block's whole length including that word, close the hole when a block is replaced or removed by lowering every later offset, and store a delete as an empty block while removing the entry.

#### Scenario: Insert, find and delete
- **WHEN** ids 5, 2 and 9 are inserted with payloads and 2 is deleted
- **THEN** the entries are 5 and 9 in order, 2 is not found and the other payloads read back unchanged

### Requirement: Locale lookup
The locale table SHALL find a name by comparing its 16 bytes against the lower-cased stored name, and SHALL return no name and id 0 for an empty file or an unknown id.

#### Scenario: Unknown or empty
- **WHEN** the table is empty
- **THEN** looking up any id returns none

### Requirement: Memory archive
The archive SHALL keep entries in file order, add an empty entry by name, replace or delete an entry by index, and read an entry's bytes by index, returning none for an index past the end.

#### Scenario: Replace
- **WHEN** an entry is added and replaced with 4 bytes
- **THEN** reading it returns those 4 bytes and other entries are unchanged

### Requirement: Project paths
A path SHALL resolve against the project folder unless it is absolute, and the user folder SHALL be chosen by the project's own rule.

#### Scenario: Relative path
- **WHEN** a relative path is resolved against a folder
- **THEN** the result is the folder joined with the path

### Requirement: Pack open rules
Opening the game data SHALL open the Resource pack by its literal stem, skip pack names it does not know and report an error for a known pack that is missing.

#### Scenario: Missing known pack
- **WHEN** a known pack file is absent from the folder
- **THEN** opening fails with an error naming it

### Requirement: Debug command registry
The registry SHALL run the first registered command whose name matches the start of the line, passing the remaining text, and SHALL report whether any command ran.

#### Scenario: First match
- **WHEN** two commands share a prefix and the line names the first
- **THEN** only the first runs, with the trailing arguments
