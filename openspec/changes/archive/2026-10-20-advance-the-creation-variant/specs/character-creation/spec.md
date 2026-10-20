## ADDED Requirements

### Requirement: Creation variant advance
Advancing SHALL read the variant from bits 20 through 23. Above 5 nothing SHALL change. Otherwise the variant SHALL rise by one and be written back into those bits, and when the widget is present its caption SHALL be the byte `(variant − 1) + 0x41` for the new variant, then the refresh SHALL run.

#### Scenario: Advance, cap and absent widget
- **WHEN** the word variant is 2 and the widget is present
- **THEN** the word variant becomes 3 and the caption is C
- **WHEN** the word variant is 5 and the widget is present
- **THEN** the word variant becomes 6 and the caption is F
- **WHEN** the word variant is 6
- **THEN** nothing changes
- **WHEN** the widget is absent
- **THEN** the word steps and no caption is chosen
