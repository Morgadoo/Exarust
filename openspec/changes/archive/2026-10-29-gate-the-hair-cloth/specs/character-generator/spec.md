## MODIFIED Requirements

### Requirement: Hair cloth
With the hair cloth switch on (`EXA_HAIR_CLOTH=1`), the character build SHALL hang each cloth-marked node of a hair style as cloth, pinned by its faces' flags, giving each style root the hair's cloth record and leaving a marked node below a root with the built-in record. With the switch off (the default) the styles stay rigid on the head.

#### Scenario: A long style
- **WHEN** a character with the first long style (`hair hula01`) is built with the switch on
- **THEN** its cloth-marked parts become particle-mode cloths with some vertices pinned and the rest free
