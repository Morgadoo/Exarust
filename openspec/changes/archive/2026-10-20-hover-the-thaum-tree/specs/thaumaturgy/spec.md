## ADDED Requirements

### Requirement: Tree hover
An event other than kind 0xae56ca01 SHALL be refused. A positive node index SHALL copy the previous entry when set, merge the event's first word into the entry's 0x700000 bits, store the event's 0x20 word, play the hover sound when ready and return true. A non-positive index SHALL copy the defaults when a previous entry is set and return false.

#### Scenario: The hover hit
- **WHEN** the kind is 0xae56ca01, the index is 3, the previous index is 2, the event's first word is 0x11, its 0x20 word is 7 and the entry's old flags are 0x123456
- **THEN** the previous entry is copied, the new flags are 0x100011 and the stored word is 7
- **AND** the hover sound plays and the result is true

#### Scenario: The miss and the guard
- **WHEN** the index is 0 and the previous index is 2
- **THEN** the defaults are copied and the result is false
- **WHEN** the kind differs
- **THEN** nothing happens
