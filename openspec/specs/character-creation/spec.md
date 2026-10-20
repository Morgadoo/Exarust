# character-creation Specification

## Purpose
The character creation screens: choosing a class and an appearance, rolling attributes, naming the character and carrying that record into a session.

## Requirements

### Requirement: Character record move
Moving SHALL copy the four words, the count, the tail and both lists into the destination, take both source lists and zero the source's count and tail, leaving the source's words unchanged.

#### Scenario: The move
- **WHEN** the source has words 1, 2, 3 and 4, count 5, parts 7 and 8, tail 9 and tail parts 10
- **THEN** the destination has the same words, count, parts, tail and tail parts
- **AND** the source has count 0, tail 0 and empty lists
- **AND** the source keeps words 1, 2, 3 and 4

### Requirement: Creation appearance label
The label SHALL be HF when bits 8 through 15 are 0 and bit 0 is set, HM when that band is 0 and bit 0 is clear, HAM for band 0x200, OM for 0x300 and ML for 0x400. Any other band SHALL have no label.

#### Scenario: The bands
- **WHEN** the word is 0x001
- **THEN** the label is HF
- **WHEN** the word is 0x000
- **THEN** the label is HM
- **WHEN** the word is 0x201
- **THEN** the label is HAM
- **WHEN** the word is 0x300
- **THEN** the label is OM
- **WHEN** the word is 0x4ff
- **THEN** the label is ML
- **WHEN** the word is 0x500
- **THEN** there is no label
