## ADDED Requirements

### Requirement: Control label horizontal alignment
The system SHALL align a label by the style bits: left of the control, centred, right-aligned, after the control, or the default half-unit inset.

#### Scenario: Four styles
- **WHEN** x is 100, the text width 20, the unit 4 and the style 0x1000
- **THEN** the label x is 76
- **WHEN** the style is 0x2000 with pad 2 and available width 100
- **THEN** the label x is 145
- **WHEN** the style is 0x3000
- **THEN** the label x is 184
- **WHEN** the style is 0x4000
- **THEN** the label x is 204

### Requirement: Control label vertical alignment
The system SHALL lift, centre, base-align, place below or use the half-unit default by the vertical style bits.

#### Scenario: Vertical styles
- **WHEN** y is 100, the unit 4 and the style 0x100
- **THEN** the label y is 98
- **WHEN** the style is 0x200 with height 20 and line height 16
- **THEN** the label y is 119
- **WHEN** the style is 0x300 with base 14
- **THEN** the label y is 133
