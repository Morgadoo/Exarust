## ADDED Requirements

### Requirement: Post quad
The same bit SHALL then draw one quad from the grip and a post. A grip of 10 and a post of 40 at unit 5 SHALL run from 11, 35 to 31, 55. A clear bit 0x200 and a clear hold bit SHALL draw none. The compare after that quad is not in.

#### Scenario: A post of 40 sits at 11, 35
- **WHEN** unit 5 dispatches a held form that is not wide, with style bits 0x80, bit 0x200, a grip of 10, and a post of 40
- **THEN** the quad runs from 11, 35 to 31, 55
