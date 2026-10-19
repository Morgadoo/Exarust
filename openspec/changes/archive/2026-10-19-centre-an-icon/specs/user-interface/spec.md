## ADDED Requirements

### Requirement: Centred icon
A centred icon SHALL sit at the origin plus half the spare, dividing a negative spare toward zero. State bit 0x10000 marks it and passes the scale bits of 1. Inside that mark, bits 0x60 equal to 0x40 add one to the height. A wide bit selects the wide draw, and an own list is kept. Otherwise bit 0x10 selects ink 0, a clear high bit with bit 0x40 selects ink 1, a clear high bit selects ink 2, a set high bit with bit 0x40 selects ink 3, and a set high bit selects ink 4. The glyph call is not in.

#### Scenario: Icon 16 sits at 52, 52
- **WHEN** icon 16 is centred in a widget at 10, 20 of size 100 by 80, and again with state bits 0x10000 and 0x40
- **THEN** the first sits at 52, 52 with ink 2, and the second sits at 52, 53 and passes the scale bits of 1
