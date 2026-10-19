## ADDED Requirements

### Requirement: Skin control image
An image of size 16 in a place at 10, 20 of size 100 by 80 SHALL be centred at 52, 52, and at 52, 53 when flags & 0x60 is 0x40. Mode 0x10000 SHALL draw with scale 1 and no ink. Otherwise the ink SHALL be slot 0xd8 for flag bit 0x10 or flags & 0x11, slot 0xf4 for flag bit 0x40, and slot 0xe4 otherwise. Style bit 0x1000000 SHALL add the raised or plain colour to the 0xf4 and 0xe4 binds. Style bits 0x70000000 SHALL select frame index (style >> 28) & 7 in every arm.

#### Scenario: The image centres and nudges
- **WHEN** an image of size 16 is drawn in a place at 10, 20 of size 100 by 80
- **THEN** its position is 52, 52
- **WHEN** flags & 0x60 is 0x40
- **THEN** its position is 52, 53

#### Scenario: The inks and frames
- **WHEN** the style is 0x10000
- **THEN** no ink is bound and the scale is 1.0
- **WHEN** flag bit 0x10 is set
- **THEN** slot 0xd8 is bound without a colour
- **WHEN** flag bit 0x40 is set and style bit 0x1000000 is set
- **THEN** slot 0xf4 is bound with the raised colour
- **WHEN** neither flag bit is set and style bit 0x1000000 is set
- **THEN** slot 0xe4 is bound with the plain colour
- **WHEN** the style is 0x30000000
- **THEN** the frame index is 3
