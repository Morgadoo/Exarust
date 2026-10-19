## ADDED Requirements

### Requirement: Skill selection settle
The selector SHALL take the low 16 bits of the raw word and clamp an index above 8 to 0. On page 1 a positive index SHALL set bits 0x110 and a zero index SHALL clear them. Any other page SHALL leave the flags unchanged.

#### Scenario: A positive index on page one
- **WHEN** the raw word is 4, the page is 1 and the flags are 0
- **THEN** the index is 4 and the flags become 0x110

#### Scenario: The clamp
- **WHEN** the raw word is 9 or -1
- **THEN** the index is 0 and the flags lose 0x110
- **WHEN** the page is not 1
- **THEN** the flags are unchanged
