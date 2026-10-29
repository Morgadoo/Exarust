## ADDED Requirements

### Requirement: Hair cloth
The character build SHALL hang each cloth-marked node of a hair style as cloth, pinned by its faces' flags, giving each style root the hair's cloth record and leaving a marked node below a root with the built-in record.

#### Scenario: A long style
- **WHEN** a character with the first long style (`hair hula01`) is built
- **THEN** its cloth-marked parts become particle-mode cloths with some vertices pinned and the rest free
