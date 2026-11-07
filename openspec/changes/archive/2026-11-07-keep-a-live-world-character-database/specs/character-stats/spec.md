## MODIFIED Requirements

### Requirement: Writing a character record
The system SHALL write a character record as the original's writer does: version 7, the fields in file order, names one byte per character, a skill only when it has a chosen technique and at most five techniques, the powers only with an entry, the origin pair only when its level word is set and the loadouts only when there is one. Every version 7 record of the shipped level databases and `actors.rpk` SHALL write back byte for byte except the voice names that are not in the formatter's form.

#### Scenario: Shipped records
- **WHEN** the 485 version 7 records of the installed levels and actors are written back
- **THEN** 483 are byte for byte the same and the other two differ only in a stored voice name `HM04` written as `vi04`
